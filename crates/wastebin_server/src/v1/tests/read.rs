//! `GET /v1/pastes/{id}` and `GET /v1/pastes/{id}/content`.

use reqwest::StatusCode;
use serde_json::{Value, json};

use super::{ORIGIN, call, create, create_owned};
use crate::test_helpers::{Client, StoreCookies};

#[tokio::test]
async fn show_plain_paste() {
    let client = Client::new(StoreCookies(false)).await;
    let created = create(&client, json!({"title": "T"})).await.json();
    let id = created["paste"]["id"].as_str().expect("id");

    let res = call(
        client.get(&format!("/v1/pastes/{id}")),
        "get",
        "/v1/pastes/{id}",
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    let paste = &res.json()["paste"];
    assert_eq!(paste["title"], "T");
    assert_eq!(paste["extension"], Value::Null);
    assert_eq!(paste["version_id"], created["paste"]["version_id"]);
    assert_eq!(
        res.header("etag").map(str::to_owned),
        Some(format!(
            "\"{}\"",
            paste["version_id"].as_str().expect("version id")
        ))
    );
    assert_eq!(paste["links"][1]["href"], format!("{ORIGIN}/{id}"));
}

#[tokio::test]
async fn show_encrypted_paste_has_no_title() {
    let client = Client::new(StoreCookies(false)).await;
    let (id, _) = create_owned(&client, json!({"title": "secret title", "password": "pw"})).await;

    let res = call(
        client.get(&format!("/v1/pastes/{id}")),
        "get",
        "/v1/pastes/{id}",
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    let paste = &res.json()["paste"];
    assert_eq!(paste["encrypted"], true);
    assert!(paste.get("title").is_none());
    assert!(!String::from_utf8_lossy(&res.body).contains("secret title"));
}

#[tokio::test]
async fn show_missing_paste_is_404() {
    let client = Client::new(StoreCookies(false)).await;

    for id in ["zzzzzz", "abc", "abc123.md"] {
        let res = call(
            client.get(&format!("/v1/pastes/{id}")),
            "get",
            "/v1/pastes/{id}",
        )
        .await;
        assert_eq!(res.status, StatusCode::NOT_FOUND, "{id}");
        assert_eq!(res.error_type(), "not_found");
    }
}

#[tokio::test]
async fn get_content_of_plain_paste() {
    let client = Client::new(StoreCookies(false)).await;
    let created = create(&client, json!({"content": "line 1\n行 2\n"}))
        .await
        .json();
    let id = created["paste"]["id"].as_str().expect("id");

    let res = call(
        client.get(&format!("/v1/pastes/{id}/content")),
        "get",
        "/v1/pastes/{id}/content",
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body, "line 1\n行 2\n".as_bytes());
    assert_eq!(
        res.header("etag").map(str::to_owned),
        Some(format!(
            "\"{}\"",
            created["paste"]["version_id"].as_str().expect("id")
        ))
    );
}

#[tokio::test]
async fn get_content_of_missing_paste_is_404() {
    let client = Client::new(StoreCookies(false)).await;
    let res = call(
        client.get("/v1/pastes/zzzzzz/content"),
        "get",
        "/v1/pastes/{id}/content",
    )
    .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_content_of_encrypted_paste_is_409_password_required() {
    let client = Client::new(StoreCookies(false)).await;

    for fields in [
        json!({"password": "pw"}),
        json!({"password": "pw", "burn_after_reading": true}),
    ] {
        let (id, _) = create_owned(&client, fields).await;
        let res = call(
            client.get(&format!("/v1/pastes/{id}/content")),
            "get",
            "/v1/pastes/{id}/content",
        )
        .await;
        assert_eq!(res.status, StatusCode::CONFLICT);
        assert_eq!(res.error_type(), "password_required");
    }
}

#[tokio::test]
async fn get_content_never_consumes_a_burn_paste() {
    let client = Client::new(StoreCookies(false)).await;
    let (id, _) = create_owned(&client, json!({"burn_after_reading": true})).await;

    for _ in 0..2 {
        let res = call(
            client.get(&format!("/v1/pastes/{id}/content")),
            "get",
            "/v1/pastes/{id}/content",
        )
        .await;
        assert_eq!(res.status, StatusCode::CONFLICT);
        assert_eq!(res.error_type(), "confirmation_required");
    }

    let res = call(
        client.get(&format!("/v1/pastes/{id}")),
        "get",
        "/v1/pastes/{id}",
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json()["paste"]["burn_after_reading"], true);

    // The paste is still there to be read once.
    let raw = client.get(&format!("/raw/{id}")).send().await.expect("raw");
    assert_eq!(raw.text().await.expect("text"), "hello");
}
