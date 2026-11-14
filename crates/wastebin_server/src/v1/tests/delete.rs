//! `DELETE /v1/pastes/{id}`.

use reqwest::StatusCode;
use serde_json::json;

use super::{ORIGIN, call, create_in_browser, create_owned};
use crate::test_helpers::{Client, StoreCookies};

#[tokio::test]
async fn delete_with_bearer_token() {
    let client = Client::new(StoreCookies(false)).await;
    let (id, token) = create_owned(&client, json!({})).await;

    let res = call(
        client
            .delete(&format!("/v1/pastes/{id}"))
            .bearer_auth(&token),
        "delete",
        "/v1/pastes/{id}",
    )
    .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);

    let res = call(
        client.get(&format!("/v1/pastes/{id}")),
        "get",
        "/v1/pastes/{id}",
    )
    .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);

    let res = call(
        client
            .delete(&format!("/v1/pastes/{id}"))
            .bearer_auth(&token),
        "delete",
        "/v1/pastes/{id}",
    )
    .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_with_cookie_needs_matching_origin() {
    let client = Client::new(StoreCookies(false)).await;
    let (id, cookie) = create_in_browser(&client).await;
    let path = format!("/v1/pastes/{id}");

    let res = call(
        client.delete(&path).header("cookie", &cookie),
        "delete",
        "/v1/pastes/{id}",
    )
    .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    let res = call(
        client
            .delete(&path)
            .header("cookie", &cookie)
            .header("origin", ORIGIN),
        "delete",
        "/v1/pastes/{id}",
    )
    .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn delete_prefers_bearer_over_cookie() {
    let client = Client::new(StoreCookies(false)).await;
    let (id, cookie) = create_in_browser(&client).await;
    let (_, other_token) = create_owned(&client, json!({})).await;

    // The cookie owns the paste, but `Authorization` alone decides.
    let res = call(
        client
            .delete(&format!("/v1/pastes/{id}"))
            .header("cookie", &cookie)
            .header("origin", ORIGIN)
            .bearer_auth(&other_token),
        "delete",
        "/v1/pastes/{id}",
    )
    .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    let res = call(
        client
            .delete(&format!("/v1/pastes/{id}"))
            .header("cookie", &cookie)
            .header("authorization", "Bearer forged"),
        "delete",
        "/v1/pastes/{id}",
    )
    .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        res.header("www-authenticate"),
        Some("Bearer realm=\"zpaste\"")
    );
}

#[tokio::test]
async fn delete_without_credentials_is_401() {
    let client = Client::new(StoreCookies(false)).await;
    let (id, _) = create_owned(&client, json!({})).await;

    for cookie in [None, Some("uid=forged")] {
        let mut request = client.delete(&format!("/v1/pastes/{id}"));
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie).header("origin", ORIGIN);
        }
        let res = call(request, "delete", "/v1/pastes/{id}").await;
        assert_eq!(res.status, StatusCode::UNAUTHORIZED, "{cookie:?}");
        assert_eq!(
            res.header("www-authenticate"),
            Some("Bearer realm=\"zpaste\"")
        );
    }
}

#[tokio::test]
async fn delete_of_another_identity_is_403() {
    let client = Client::new(StoreCookies(false)).await;
    let (id, _) = create_owned(&client, json!({})).await;
    let (_, other_token) = create_owned(&client, json!({})).await;

    let res = call(
        client
            .delete(&format!("/v1/pastes/{id}"))
            .bearer_auth(&other_token),
        "delete",
        "/v1/pastes/{id}",
    )
    .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    assert_eq!(res.error_type(), "forbidden");

    let res = call(
        client.get(&format!("/v1/pastes/{id}")),
        "get",
        "/v1/pastes/{id}",
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
}
