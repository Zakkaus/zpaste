//! `GET /v1/pastes?owner=me`.

use reqwest::StatusCode;
use serde_json::{Value, json};

use super::{Checked, ORIGIN, call, create_in_browser, create_owned, paste};
use crate::test_helpers::{Client, StoreCookies};

async fn list(client: &Client, query: &str, credential: Option<(&str, &str)>) -> Checked {
    let mut request = client.get(&format!("/v1/pastes?{query}"));
    if let Some((name, value)) = credential {
        request = request.header(name, value);
    }
    call(request, "get", "/v1/pastes").await
}

async fn create_with_token(client: &Client, token: &str, fields: Value) -> String {
    let res = call(
        client
            .post("/v1/pastes")
            .bearer_auth(token)
            .json(&paste(fields)),
        "post",
        "/v1/pastes",
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    res.json()["paste"]["id"].as_str().expect("id").to_owned()
}

fn ids(res: &Checked) -> Vec<String> {
    res.json()["pastes"]
        .as_array()
        .expect("pastes")
        .iter()
        .map(|paste| paste["id"].as_str().expect("id").to_owned())
        .collect()
}

#[tokio::test]
async fn lists_own_pastes_page_by_page() {
    let client = Client::new(StoreCookies(false)).await;
    let (first, token) = create_owned(&client, json!({"title": "one"})).await;
    let mut mine = vec![first];
    for n in 0..3 {
        mine.push(create_with_token(&client, &token, json!({"title": format!("t{n}")})).await);
    }
    let encrypted = create_with_token(
        &client,
        &token,
        json!({"title": "hidden", "password": "pw"}),
    )
    .await;
    mine.push(encrypted.clone());
    let (_, _) = create_owned(&client, json!({"title": "someone else"})).await;
    let bearer = format!("Bearer {token}");
    let auth = Some(("authorization", bearer.as_str()));

    // The default limit holds all five; the encrypted paste carries no title.
    let all = list(&client, "owner=me", auth).await;
    assert_eq!(all.status, StatusCode::OK);
    assert_eq!(all.json()["pastes_links"], json!([]));
    let mut listed = ids(&all);
    listed.sort();
    mine.sort();
    assert_eq!(listed, mine);
    let hidden = all.json()["pastes"]
        .as_array()
        .expect("pastes")
        .iter()
        .find(|paste| paste["id"] == encrypted.as_str())
        .cloned()
        .expect("encrypted paste listed");
    assert_eq!(hidden.get("title"), None);
    assert_eq!(hidden["encrypted"], true);

    // Pages of two follow `next`, keep `owner=me` and `limit`, and add up to the full list.
    let mut paged = Vec::new();
    let mut query = String::from("owner=me&limit=2");
    loop {
        let page = list(&client, &query, auth).await;
        assert_eq!(page.status, StatusCode::OK);
        assert!(ids(&page).len() <= 2);
        paged.extend(ids(&page));
        let Some(next) = page.json()["pastes_links"][0]["href"]
            .as_str()
            .map(str::to_owned)
        else {
            break;
        };
        assert_eq!(page.json()["pastes_links"][0]["rel"], "next");
        let rest = next
            .strip_prefix(&format!("{ORIGIN}/v1/pastes?"))
            .expect("next link on this server");
        assert!(rest.starts_with("owner=me&limit=2&marker="), "{rest}");
        query = rest.to_owned();
    }
    assert_eq!(paged, ids(&all));
}

#[tokio::test]
async fn cookie_lists_every_identity_it_holds() {
    let client = Client::new(StoreCookies(false)).await;
    let (browser_id, cookie) = create_in_browser(&client).await;
    let (token_id, token) = create_owned(&client, json!({})).await;

    // Claiming the token adds its identity to the cookie (upstream's `?owner=` hand-off).
    let res = client
        .get(&format!("/{token_id}"))
        .query(&[("owner", &token)])
        .header("cookie", &cookie)
        .send()
        .await
        .expect("claim");
    let both = res
        .headers()
        .get("set-cookie")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .expect("claimed cookie")
        .to_owned();

    let res = list(&client, "owner=me", Some(("cookie", &both))).await;
    assert_eq!(res.status, StatusCode::OK);
    let mut listed = ids(&res);
    listed.sort();
    let mut expected = vec![browser_id, token_id];
    expected.sort();
    assert_eq!(listed, expected);
}

#[tokio::test]
async fn list_needs_credentials() {
    let client = Client::new(StoreCookies(false)).await;
    for credential in [None, Some(("authorization", "Bearer forged"))] {
        let res = list(&client, "owner=me", credential).await;
        assert_eq!(res.status, StatusCode::UNAUTHORIZED);
        assert_eq!(res.error_type(), "unauthorized");
        assert_eq!(
            res.header("www-authenticate"),
            Some("Bearer realm=\"zpaste\"")
        );
    }
}

#[tokio::test]
async fn invalid_queries_are_400() {
    let client = Client::new(StoreCookies(false)).await;
    let (_, token) = create_owned(&client, json!({})).await;
    let (theirs, _) = create_owned(&client, json!({})).await;
    let bearer = format!("Bearer {token}");
    let auth = Some(("authorization", bearer.as_str()));

    let mut markers = Vec::new();
    for query in [
        String::from(""),
        String::from("owner=you"),
        String::from("owner=me&limit=0"),
        String::from("owner=me&limit=101"),
        String::from("owner=me&limit=ten"),
        String::from("owner=me&marker=bad"),
        // An expired marker is the same refusal; `wastebin_core` tests that case, which needs a
        // paste to age past its expiry.
        format!("owner=me&marker={theirs}"),
        String::from("owner=me&marker=abc123"),
    ] {
        let res = list(&client, &query, auth).await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{query}");
        assert_eq!(res.error_type(), "bad_request");
        if query.contains("marker") {
            markers.push(res.json()["error"]["message"].clone());
        }
    }
    // Every refused marker reads the same, so none reveals whether its id exists.
    markers.dedup();
    assert_eq!(markers.len(), 1, "{markers:?}");
}
