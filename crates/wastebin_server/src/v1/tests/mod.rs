//! Contract tests: every built operation with each status it chooses, validated against the
//! generated document (status, required headers and their schemas, body schema).

use std::sync::LazyLock;

use reqwest::{RequestBuilder, Response, StatusCode};
use serde_json::{Value, json};

use crate::test_helpers::{Client, StoreCookies};
use crate::v1::contract::{document, schema};

mod create;
mod delete;
mod read;

/// The public URL of the test server; a request with this `Origin` is a browser request.
const ORIGIN: &str = "https://localhost:8888";

static DOCUMENT: LazyLock<Value> = LazyLock::new(document);

/// A response read in full and checked against the operation `method path_template`.
struct Checked {
    status: StatusCode,
    headers: reqwest::header::HeaderMap,
    body: Vec<u8>,
}

impl Checked {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("JSON body")
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|value| value.to_str().ok())
    }

    fn error_type(&self) -> String {
        self.json()["error"]["type"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    }
}

fn resolve(value: &Value) -> &Value {
    match value.get("$ref").and_then(Value::as_str) {
        Some(reference) => schema::resolve(&DOCUMENT, reference).expect("resolvable $ref"),
        None => value,
    }
}

/// Send `request` and validate the answer against `method path_template` of the contract.
async fn call(request: RequestBuilder, method: &str, path_template: &str) -> Checked {
    let response: Response = request.send().await.expect("request");
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.bytes().await.expect("body").to_vec();
    let checked = Checked {
        status,
        headers,
        body,
    };

    let operation = &DOCUMENT["paths"][path_template][method];
    assert!(
        operation.is_object(),
        "{method} {path_template} is not in the contract"
    );
    let responses = &operation["responses"];
    let spec = responses
        .get(status.as_str())
        .or_else(|| responses.get("default"))
        .unwrap_or_else(|| unreachable!("{method} {path_template} lists no {status}"));
    let spec = resolve(spec);

    for (name, header) in spec["headers"].as_object().into_iter().flatten() {
        let header = resolve(header);
        match checked.header(name) {
            Some(value) => {
                let instance = if resolve(&header["schema"])["type"] == "integer" {
                    json!(value.parse::<i64>().expect("integer header"))
                } else {
                    json!(value)
                };
                schema::validate(&DOCUMENT, &header["schema"], &instance)
                    .unwrap_or_else(|err| unreachable!("header {name}: {err}"));
            }
            None => assert_ne!(
                header["required"], true,
                "{method} {path_template} {status}: missing header {name}"
            ),
        }
    }

    match spec["content"].as_object() {
        Some(content) => {
            let content_type = checked.header("content-type").unwrap_or_default();
            let (media, media_spec) = content
                .iter()
                .find(|(media, _)| media_matches(content_type, media))
                .unwrap_or_else(|| unreachable!("content type {content_type} not in {content:?}"));
            let instance = if media.starts_with("application/json") {
                checked.json()
            } else {
                json!(String::from_utf8(checked.body.clone()).expect("UTF-8 body"))
            };
            schema::validate(&DOCUMENT, &media_spec["schema"], &instance)
                .unwrap_or_else(|err| unreachable!("{method} {path_template} {status}: {err}"));
        }
        None => assert!(checked.body.is_empty(), "{status} has an undocumented body"),
    }

    if status.is_client_error() || status.is_server_error() {
        let error = &checked.json()["error"];
        assert_eq!(error["code"], status.as_u16());
        assert_eq!(
            error["request_id"],
            checked.header("x-request-id").unwrap_or_default()
        );
    }

    checked
}

/// `content_type` is `media`, or `media` names only the type and `content_type` adds parameters.
fn media_matches(content_type: &str, media: &str) -> bool {
    let content_type = content_type.replace(' ', "");
    let media = media.replace(' ', "");
    content_type == media
        || (!media.contains(';') && content_type.split(';').next() == Some(&media))
}

fn paste(fields: Value) -> Value {
    let mut paste = json!({"content": "hello"});
    if let (Some(paste), Some(fields)) = (paste.as_object_mut(), fields.as_object()) {
        paste.extend(fields.clone());
    }
    json!({"paste": paste})
}

async fn create(client: &Client, fields: Value) -> Checked {
    call(
        client.post("/v1/pastes").json(&paste(fields)),
        "post",
        "/v1/pastes",
    )
    .await
}

/// Create a paste without credentials; returns its id and owner token.
async fn create_owned(client: &Client, fields: Value) -> (String, String) {
    let created = create(client, fields).await;
    assert_eq!(created.status, StatusCode::CREATED);
    let body = created.json();
    (
        body["paste"]["id"].as_str().expect("id").to_owned(),
        body["paste"]["owner_token"]
            .as_str()
            .expect("owner token")
            .to_owned(),
    )
}

/// Create a paste as the web app does; returns its id and the `uid` cookie pair.
async fn create_in_browser(client: &Client) -> (String, String) {
    let created = call(
        client
            .post("/v1/pastes")
            .header("origin", ORIGIN)
            .json(&paste(json!({}))),
        "post",
        "/v1/pastes",
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED);
    assert!(created.json()["paste"].get("owner_token").is_none());

    let set_cookie = created.header("set-cookie").expect("Set-Cookie");
    for attribute in ["HttpOnly", "Secure", "SameSite=Lax", "Path=/"] {
        assert!(
            set_cookie.contains(attribute),
            "{set_cookie} lacks {attribute}"
        );
    }
    let pair = set_cookie
        .split(';')
        .next()
        .expect("cookie pair")
        .to_owned();
    (
        created.json()["paste"]["id"]
            .as_str()
            .expect("id")
            .to_owned(),
        pair,
    )
}

#[tokio::test]
async fn get_version() {
    let client = Client::new(StoreCookies(false)).await;
    let res = call(client.get("/v1"), "get", "/v1").await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(
        res.json(),
        json!({"version": {"id": "v1", "status": "CURRENT"}})
    );
}

#[tokio::test]
async fn request_id_is_kept_when_well_formed_and_replaced_otherwise() {
    let client = Client::new(StoreCookies(false)).await;

    let res = call(
        client.get("/v1").header("x-request-id", "abc-123"),
        "get",
        "/v1",
    )
    .await;
    assert_eq!(res.header("x-request-id"), Some("abc-123"));

    let res = call(
        client.get("/v1").header("x-request-id", "bad id!"),
        "get",
        "/v1",
    )
    .await;
    let replaced = res.header("x-request-id").expect("request id");
    assert_ne!(replaced, "bad id!");

    let res = call(
        client
            .get("/v1/pastes/zzzzzz")
            .header("x-request-id", "x".repeat(65)),
        "get",
        "/v1/pastes/{id}",
    )
    .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(res.header("x-request-id").map(str::len), Some(32));
}

#[tokio::test]
async fn unknown_v1_path_is_404_in_the_envelope() {
    let client = Client::new(StoreCookies(false)).await;
    let res = client
        .get("/v1/nothing/here")
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        res.headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok()),
        Some("no-store")
    );
    let body: Value = res.json().await.expect("JSON");
    assert_eq!(body["error"]["type"], "not_found");
}

#[tokio::test]
async fn new_paste_page_is_html() {
    let client = Client::new(StoreCookies(false)).await;
    let res = call(client.get("/"), "get", "/").await;
    assert_eq!(res.status, StatusCode::OK);
    assert!(
        String::from_utf8_lossy(&res.body).contains("<meta name=\"generator\" content=\"Zpaste "),
        "the page names Zpaste as its generator"
    );
}
