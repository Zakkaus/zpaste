//! `POST /v1/pastes`.

use reqwest::StatusCode;
use serde_json::{Value, json};

use super::{ORIGIN, call, create, create_in_browser, create_owned, paste};
use crate::test_helpers::{Client, StoreCookies};

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
