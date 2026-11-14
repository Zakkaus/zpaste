//! `POST /v1/pastes`.

use reqwest::StatusCode;
use serde_json::{Value, json};

use std::num::NonZeroUsize;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wastebin_core::crypto::KdfLimit;

use super::{Checked, ORIGIN, call, check, create, create_in_browser, create_owned, paste};
use crate::test_helpers::{Client, Settings, StoreCookies};

#[tokio::test]
async fn create_without_credentials_returns_owner_token() {
    let client = Client::new(StoreCookies(false)).await;
    let res = create(
        &client,
        json!({"extension": "md", "title": "Plan", "burn_after_reading": false, "password": null}),
    )
    .await;

    assert_eq!(res.status, StatusCode::CREATED);
    assert!(res.header("set-cookie").is_none());
    let paste = &res.json()["paste"];
    let id = paste["id"].as_str().expect("id");
    assert_eq!(
        res.header("location"),
        Some(format!("/v1/pastes/{id}").as_str())
    );
    assert_eq!(
        res.header("etag"),
        Some(format!("\"{}\"", paste["version_id"].as_str().expect("version id")).as_str())
    );
    assert!(
        paste["owner_token"]
            .as_str()
            .is_some_and(|token| !token.is_empty())
    );
    assert_eq!(paste["extension"], "md");
    assert_eq!(paste["title"], "Plan");
    assert_eq!(paste["version"], 1);
    assert_eq!(paste["expires_at"], Value::Null);
    assert_eq!(paste["links"][1]["href"], format!("{ORIGIN}/{id}.md"));

    // The legacy raw route reads the same paste.
    let raw = client.get(&format!("/raw/{id}")).send().await.expect("raw");
    assert_eq!(raw.text().await.expect("text"), "hello");
}

#[tokio::test]
async fn browser_create_sets_cookie_and_hides_token() {
    let client = Client::new(StoreCookies(false)).await;
    create_in_browser(&client).await;
}

#[tokio::test]
async fn create_with_bearer_token_joins_its_identity() {
    let client = Client::new(StoreCookies(false)).await;
    let (_, token) = create_owned(&client, json!({})).await;

    let res = call(
        client
            .post("/v1/pastes")
            .bearer_auth(&token)
            .json(&paste(json!({}))),
        "post",
        "/v1/pastes",
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert!(res.json()["paste"].get("owner_token").is_none());
    assert!(res.header("set-cookie").is_none());

    // The same token owns the second paste.
    let id = res.json()["paste"]["id"].as_str().expect("id").to_owned();
    let res = call(
        client
            .delete(&format!("/v1/pastes/{id}"))
            .bearer_auth(&token),
        "delete",
        "/v1/pastes/{id}",
    )
    .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn create_rejects_invalid_bodies_with_400() {
    let client = Client::new(StoreCookies(false)).await;

    let bodies = [
        json!({"paste": {"content": "x", "unknown": 1}}),
        json!({"content": "x"}),
        json!({"paste": {}}),
        json!({"paste": {"content": 1}}),
        paste(json!({"extension": "\"><img"})),
        paste(json!({"extension": ""})),
        paste(json!({"title": "x".repeat(257)})),
        paste(json!({"password": ""})),
        paste(json!({"expires_in": 60})),
        paste(json!({"expires_in": -1})),
    ];

    for body in bodies {
        let res = call(client.post("/v1/pastes").json(&body), "post", "/v1/pastes").await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(res.error_type(), "bad_request");
    }

    let res = call(
        client
            .post("/v1/pastes")
            .header("content-type", "application/json")
            .body("{not json"),
        "post",
        "/v1/pastes",
    )
    .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_accepts_the_configured_expiration() {
    let client = Client::new(StoreCookies(false)).await;
    let res = create(&client, json!({"expires_in": 0, "title": "x".repeat(256)})).await;
    assert_eq!(res.status, StatusCode::CREATED);
}

#[tokio::test]
async fn create_answers_framework_rejections_in_the_envelope() {
    let client = Client::new(StoreCookies(false)).await;

    let res = call(client.post("/v1/pastes").body("{}"), "post", "/v1/pastes").await;
    assert_eq!(res.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(res.error_type(), "unsupported_media_type");
    // The handler's message survives: the HTML error pages leave `/v1` alone.
    assert_eq!(
        res.json()["error"]["message"],
        "expected Content-Type: application/json"
    );

    let big = paste(json!({"content": "x".repeat(1024 * 1024)}));
    let res = call(client.post("/v1/pastes").json(&big), "post", "/v1/pastes").await;
    assert_eq!(res.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(res.error_type(), "payload_too_large");

    let res = call(client.put("/v1/pastes"), "post", "/v1/pastes").await;
    assert_eq!(res.status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(res.error_type(), "method_not_allowed");
}

#[tokio::test]
async fn create_with_invalid_bearer_is_401() {
    let client = Client::new(StoreCookies(false)).await;

    for authorization in ["Bearer forged", "Basic dXNlcjpwYXNz", "Bearer"] {
        let res = call(
            client
                .post("/v1/pastes")
                .header("authorization", authorization)
                .json(&paste(json!({}))),
            "post",
            "/v1/pastes",
        )
        .await;
        assert_eq!(res.status, StatusCode::UNAUTHORIZED, "{authorization}");
        assert_eq!(res.error_type(), "unauthorized");
        assert_eq!(
            res.header("www-authenticate"),
            Some("Bearer realm=\"zpaste\"")
        );
    }
}

#[tokio::test]
async fn create_with_cookie_needs_matching_origin() {
    let client = Client::new(StoreCookies(false)).await;
    let (_, cookie) = create_in_browser(&client).await;

    for origin in [None, Some("null"), Some("https://evil.example")] {
        let mut request = client
            .post("/v1/pastes")
            .header("cookie", &cookie)
            .json(&paste(json!({})));
        if let Some(origin) = origin {
            request = request.header("origin", origin);
        }
        let res = call(request, "post", "/v1/pastes").await;
        assert_eq!(res.status, StatusCode::FORBIDDEN, "{origin:?}");
        assert_eq!(res.error_type(), "forbidden");
    }

    // With the right Origin the cookie's identity creates, and no new cookie is set.
    let res = call(
        client
            .post("/v1/pastes")
            .header("cookie", &cookie)
            .header("origin", ORIGIN)
            .json(&paste(json!({}))),
        "post",
        "/v1/pastes",
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert!(res.header("set-cookie").is_none());
    assert!(res.json()["paste"].get("owner_token").is_none());
}

#[tokio::test]
async fn create_with_stale_cookie_mints_a_new_identity() {
    let client = Client::new(StoreCookies(false)).await;
    let res = call(
        client
            .post("/v1/pastes")
            .header("cookie", "uid=forged")
            .header("origin", ORIGIN)
            .json(&paste(json!({}))),
        "post",
        "/v1/pastes",
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert!(res.header("set-cookie").is_some());
}

#[tokio::test]
async fn create_with_password_is_429_while_every_derivation_slot_is_taken() {
    let kdf = KdfLimit::new(NonZeroUsize::MIN, Duration::ZERO);
    let client = Client::with_settings(
        StoreCookies(false),
        Settings {
            kdf: kdf.clone(),
            ..Settings::default()
        },
    )
    .await;
    let _held = kdf.acquire().await.expect("free slot");

    let res = create(&client, json!({"password": "hunter2"})).await;
    assert_eq!(res.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(res.error_type(), "rate_limited");
    assert_eq!(res.header("retry-after"), Some("1"));

    // Credential failures are answered before any derivation.
    let res = call(
        client
            .post("/v1/pastes")
            .bearer_auth("forged")
            .json(&paste(json!({"password": "hunter2"}))),
        "post",
        "/v1/pastes",
    )
    .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);

    // A create without a password derives nothing, so the limit does not apply.
    let res = create(&client, json!({})).await;
    assert_eq!(res.status, StatusCode::CREATED);
}

#[tokio::test]
async fn timeout_is_408_in_the_envelope() {
    let client = Client::with_settings(
        StoreCookies(false),
        Settings {
            timeout: Duration::from_millis(100),
            ..Settings::default()
        },
    )
    .await;

    // Announce a body and send only part of it, so the request outlives the timeout.
    let mut socket = tokio::net::TcpStream::connect(client.addr())
        .await
        .expect("connect");
    socket
        .write_all(
            b"POST /v1/pastes HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\
              Content-Type: application/json\r\nContent-Length: 64\r\n\r\n{\"paste\"",
        )
        .await
        .expect("write");
    let mut raw = Vec::new();
    tokio::time::timeout(Duration::from_secs(10), socket.read_to_end(&mut raw))
        .await
        .expect("answered within 10 s")
        .expect("read");

    let res = check(parse_response(&raw), "post", "/v1/pastes");
    assert_eq!(res.status, StatusCode::REQUEST_TIMEOUT);
    assert_eq!(res.error_type(), "timeout");
}

/// An HTTP/1.1 response with a `Content-Length` body, as read from a raw socket.
fn parse_response(raw: &[u8]) -> Checked {
    let split = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("end of head");
    let head = std::str::from_utf8(&raw[..split]).expect("ASCII head");
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .and_then(|code| StatusCode::from_u16(code).ok())
        .expect("status line");

    let mut headers = reqwest::header::HeaderMap::new();
    for line in lines {
        let (name, value) = line.split_once(':').expect("header line");
        headers.append(
            reqwest::header::HeaderName::try_from(name).expect("header name"),
            reqwest::header::HeaderValue::try_from(value.trim()).expect("header value"),
        );
    }

    Checked {
        status,
        headers,
        body: raw[split + 4..].to_vec(),
    }
}
