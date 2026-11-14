//! `POST /v1/pastes/{id}/content`: password, burn confirmation, atomic consumption and the
//! per-client derivation rate.

use std::num::{NonZeroU32, NonZeroUsize};
use std::time::Duration;

use reqwest::StatusCode;
use serde_json::{Value, json};
use wastebin_core::crypto::KdfLimit;

use super::{Checked, ORIGIN, call, create, create_in_browser};
use crate::client::{RateLimit, TrustedProxies};
use crate::test_helpers::{Client, Settings, StoreCookies};

const PATH: &str = "/v1/pastes/{id}/content";

async fn read(client: &Client, id: &str, body: Value) -> Checked {
    read_with(client, id, body, &[]).await
}

async fn read_with(client: &Client, id: &str, body: Value, headers: &[(&str, &str)]) -> Checked {
    let mut request = client.post(&format!("/v1/pastes/{id}/content")).json(&body);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    call(request, "post", PATH).await
}

async fn created_id(client: &Client, fields: Value) -> String {
    let created = create(client, fields).await;
    assert_eq!(created.status, StatusCode::CREATED);
    created.json()["paste"]["id"]
        .as_str()
        .expect("id")
        .to_owned()
}

fn rate(per_minute: u32) -> Settings {
    Settings {
        rate: RateLimit::per_minute(NonZeroU32::new(per_minute).expect("non-zero")),
        ..Settings::default()
    }
}

#[tokio::test]
async fn plain_paste_reads_with_an_empty_body() {
    let client = Client::new(StoreCookies(false)).await;
    let created = create(&client, json!({"content": "plain"})).await.json();
    let id = created["paste"]["id"].as_str().expect("id");

    let res = read(&client, id, json!({})).await;
    assert_eq!(res.status, StatusCode::OK);
    let version_id = &created["paste"]["version_id"];
    assert_eq!(
        res.json(),
        json!({"content": {"text": "plain", "version_id": version_id}})
    );
    assert_eq!(
        res.header("etag").map(str::to_owned),
        Some(format!("\"{}\"", version_id.as_str().expect("version id")))
    );
}

#[tokio::test]
async fn encrypted_paste_needs_the_right_password() {
    let client = Client::new(StoreCookies(false)).await;
    let id = created_id(&client, json!({"content": "secret", "password": "right"})).await;

    let res = read(&client, &id, json!({})).await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.error_type(), "password_required");

    let res = read(&client, &id, json!({"password": "wrong"})).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert_eq!(res.error_type(), "wrong_password");
    assert_eq!(
        res.header("www-authenticate"),
        Some("Paste-Password realm=\"zpaste\"")
    );

    let res = read(&client, &id, json!({"password": "right"})).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json()["content"]["text"], "secret");
    // Not burn-after-reading, so it stays.
    let res = read(&client, &id, json!({"password": "right"})).await;
    assert_eq!(res.status, StatusCode::OK);
}

#[tokio::test]
async fn burn_paste_is_consumed_only_after_confirmation() {
    let client = Client::new(StoreCookies(false)).await;
    let id = created_id(
        &client,
        json!({"content": "once", "burn_after_reading": true}),
    )
    .await;

    let res = read(&client, &id, json!({})).await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.error_type(), "confirmation_required");

    let res = read(&client, &id, json!({"confirm_burn": true})).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json()["content"]["text"], "once");

    let res = read(&client, &id, json!({"confirm_burn": true})).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(res.error_type(), "not_found");
}

#[tokio::test]
async fn encrypted_burn_paste_checks_the_password_before_the_confirmation() {
    let client = Client::new(StoreCookies(false)).await;
    let id = created_id(
        &client,
        json!({"content": "both", "password": "right", "burn_after_reading": true}),
    )
    .await;

    let res = read(
        &client,
        &id,
        json!({"password": "wrong", "confirm_burn": true}),
    )
    .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);

    // The right password alone reports the confirmation and consumes nothing.
    let res = read(&client, &id, json!({"password": "right"})).await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.error_type(), "confirmation_required");
    assert_eq!(res.json()["error"].get("text"), None);

    let res = read(
        &client,
        &id,
        json!({"password": "right", "confirm_burn": true}),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json()["content"]["text"], "both");

    let res = read(
        &client,
        &id,
        json!({"password": "right", "confirm_burn": true}),
    )
    .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}

/// Of concurrent confirmed reads of one burn paste, exactly one gets the content.
async fn consumed_once(client: &Client, id: &str, body: Value, readers: usize) {
    let mut reads = tokio::task::JoinSet::new();
    for _ in 0..readers {
        let request = client.post(&format!("/v1/pastes/{id}/content")).json(&body);
        reads.spawn(call(request, "post", PATH));
    }
    let statuses = reads
        .join_all()
        .await
        .into_iter()
        .map(|checked| checked.status)
        .collect::<Vec<_>>();

    let ok = statuses.iter().filter(|s| **s == StatusCode::OK).count();
    let gone = statuses
        .iter()
        .filter(|s| **s == StatusCode::NOT_FOUND)
        .count();
    assert_eq!((ok, gone), (1, readers - 1), "{statuses:?}");
}

#[tokio::test]
async fn concurrent_confirmations_consume_a_burn_paste_once() {
    let client = Client::new(StoreCookies(false)).await;
    let id = created_id(
        &client,
        json!({"content": "once", "burn_after_reading": true}),
    )
    .await;
    consumed_once(&client, &id, json!({"confirm_burn": true}), 16).await;
}

#[tokio::test]
async fn concurrent_confirmations_consume_an_encrypted_burn_paste_once() {
    let client = Client::with_settings(
        StoreCookies(false),
        Settings {
            kdf: KdfLimit::new(
                NonZeroUsize::new(4).expect("non-zero"),
                Duration::from_secs(60),
            ),
            ..Settings::default()
        },
    )
    .await;
    let id = created_id(
        &client,
        json!({"content": "both", "password": "pw", "burn_after_reading": true}),
    )
    .await;
    consumed_once(
        &client,
        &id,
        json!({"password": "pw", "confirm_burn": true}),
        4,
    )
    .await;
}

#[tokio::test]
async fn malformed_bodies_are_400() {
    let client = Client::new(StoreCookies(false)).await;
    let id = created_id(&client, json!({"content": "x", "burn_after_reading": true})).await;

    for body in [
        json!({"password": ""}),
        json!({"confirm_burn": false}),
        json!({"confirm_burn": "yes"}),
        json!({"unknown": 1}),
        json!([]),
    ] {
        let res = read(&client, &id, body.clone()).await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(res.error_type(), "bad_request");
    }

    // Nothing above consumed the paste.
    let res = read(&client, &id, json!({"confirm_burn": true})).await;
    assert_eq!(res.status, StatusCode::OK);
}

#[tokio::test]
async fn missing_paste_is_404() {
    let client = Client::new(StoreCookies(false)).await;
    for id in ["abc123", "bad", "abc123.md"] {
        let res = read(&client, id, json!({"confirm_burn": true})).await;
        assert_eq!(res.status, StatusCode::NOT_FOUND, "{id}");
    }
}

#[tokio::test]
async fn cookie_without_origin_is_403_before_the_paste_or_a_derivation() {
    // Two tokens: the encrypted create spends one.
    let client = Client::with_settings(StoreCookies(false), rate(2)).await;
    let (_, cookie) = create_in_browser(&client).await;
    let id = created_id(&client, json!({"content": "x", "password": "pw"})).await;

    for target in ["abc123", id.as_str()] {
        let res = read_with(
            &client,
            target,
            json!({"password": "wrong"}),
            &[("cookie", &cookie)],
        )
        .await;
        assert_eq!(res.status, StatusCode::FORBIDDEN, "{target}");
        assert_eq!(res.error_type(), "forbidden");
    }

    // The 403s spent no derivation: the last token of this client is still there.
    let res = read_with(
        &client,
        &id,
        json!({"password": "wrong"}),
        &[("cookie", &cookie), ("origin", ORIGIN)],
    )
    .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn client_past_its_derivation_rate_is_429() {
    // Two tokens: the encrypted create spends one, the first read the other.
    let client = Client::with_settings(StoreCookies(false), rate(2)).await;
    let id = created_id(&client, json!({"content": "x", "password": "pw"})).await;

    let res = read(&client, &id, json!({"password": "wrong"})).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);

    for body in [json!({"password": "wrong"}), json!({"password": "pw"})] {
        let res = read(&client, &id, body).await;
        assert_eq!(res.status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(res.error_type(), "rate_limited");
        let retry_after = res
            .header("retry-after")
            .and_then(|value| value.parse::<u64>().ok())
            .expect("Retry-After");
        assert!((29..=30).contains(&retry_after), "{retry_after}");
    }

    // Requests that derive nothing are not limited.
    let res = read(&client, &id, json!({})).await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    let plain = created_id(&client, json!({"content": "plain"})).await;
    let res = read(&client, &plain, json!({"password": "ignored"})).await;
    assert_eq!(res.status, StatusCode::OK);
}

#[tokio::test]
async fn busy_derivation_slots_are_429() {
    let kdf = KdfLimit::new(NonZeroUsize::MIN, Duration::ZERO);
    let client = Client::with_settings(
        StoreCookies(false),
        Settings {
            kdf: kdf.clone(),
            ..Settings::default()
        },
    )
    .await;
    let id = created_id(&client, json!({"content": "x", "password": "pw"})).await;

    let _held = kdf.acquire().await.expect("free slot");
    let res = read(&client, &id, json!({"password": "pw"})).await;
    assert_eq!(res.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(res.header("retry-after"), Some("1"));
}

#[tokio::test]
async fn cf_connecting_ip_counts_only_from_a_trusted_proxy() {
    // Without trusted proxies the header is ignored: the create and both reads share the two
    // tokens of 127.0.0.1.
    let client = Client::with_settings(StoreCookies(false), rate(2)).await;
    let id = created_id(&client, json!({"content": "x", "password": "pw"})).await;
    let mut statuses = Vec::new();
    for ip in ["198.51.100.1", "198.51.100.2"] {
        let res = read_with(
            &client,
            &id,
            json!({"password": "wrong"}),
            &[("cf-connecting-ip", ip)],
        )
        .await;
        statuses.push(res.status);
    }
    assert_eq!(
        statuses,
        [StatusCode::UNAUTHORIZED, StatusCode::TOO_MANY_REQUESTS]
    );

    // Behind a trusted proxy each forwarded client has its own bucket; the create, without the
    // header, spends the proxy's own.
    let client = Client::with_settings(
        StoreCookies(false),
        Settings {
            proxies: "127.0.0.0/8".parse::<TrustedProxies>().expect("ranges"),
            ..rate(1)
        },
    )
    .await;
    let id = created_id(&client, json!({"content": "x", "password": "pw"})).await;
    let mut statuses = Vec::new();
    for ip in ["198.51.100.1", "198.51.100.2", "198.51.100.1"] {
        let res = read_with(
            &client,
            &id,
            json!({"password": "wrong"}),
            &[("cf-connecting-ip", ip)],
        )
        .await;
        statuses.push(res.status);
    }
    assert_eq!(
        statuses,
        [
            StatusCode::UNAUTHORIZED,
            StatusCode::UNAUTHORIZED,
            StatusCode::TOO_MANY_REQUESTS
        ]
    );
}
