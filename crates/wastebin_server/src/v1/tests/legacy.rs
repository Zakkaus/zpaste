//! The legacy routes are adapters over the same service as `/v1` (plan section 四, 舊路由與契約):
//! one set of scenarios runs through every pair of create and read route and must end the same.

use std::collections::HashMap;
use std::num::NonZeroU32;

use reqwest::StatusCode;
use serde_json::{Value, json};

use super::{call, create};
use crate::client::RateLimit;
use crate::handlers::extract::PASSWORD_HEADER_NAME;
use crate::test_helpers::{Client, Settings, StoreCookies};

#[derive(Clone, Copy, Debug)]
enum Create {
    /// `POST /` with JSON.
    Legacy,
    /// `POST /new` with the HTML form.
    Form,
    V1,
}

#[derive(Clone, Copy, Debug)]
enum Read {
    /// `GET /raw/{id}`, which consumes a burn paste as upstream did.
    Raw,
    /// `GET /dl/{id}`, which consumes like `/raw`.
    Download,
    /// `POST /{id}`, the HTML page's form, with the burn confirmed.
    Html,
    /// `POST /v1/pastes/{id}/content` with `confirm_burn: true`.
    V1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Outcome {
    Text(String),
    PasswordRequired,
    WrongPassword,
    NotFound,
}

struct Paste {
    text: &'static str,
    password: Option<&'static str>,
    burn: bool,
}

async fn create_with(client: &Client, route: Create, paste: &Paste) -> String {
    match route {
        Create::Legacy => {
            let res = client
                .post_json()
                .json(&json!({
                    "text": paste.text,
                    "password": paste.password,
                    "burn_after_reading": paste.burn,
                }))
                .send()
                .await
                .expect("legacy create");
            assert_eq!(res.status(), StatusCode::OK);
            let body = res.json::<Value>().await.expect("legacy body");
            body["path"]
                .as_str()
                .expect("path")
                .trim_start_matches('/')
                .to_owned()
        }
        Create::Form => {
            let mut form = HashMap::from([
                ("text", paste.text),
                ("password", paste.password.unwrap_or_default()),
                ("title", ""),
            ]);
            if paste.burn {
                form.insert("burn-after-reading", "on");
            }
            let res = client.post_form().form(&form).send().await.expect("form");
            assert_eq!(res.status(), StatusCode::SEE_OTHER);
            let location = res
                .headers()
                .get("location")
                .and_then(|value| value.to_str().ok())
                .expect("location");
            location.rsplit('/').next().expect("id").to_owned()
        }
        Create::V1 => {
            let res = create(
                client,
                json!({
                    "content": paste.text,
                    "password": paste.password,
                    "burn_after_reading": paste.burn,
                }),
            )
            .await;
            assert_eq!(res.status, StatusCode::CREATED);
            res.json()["paste"]["id"].as_str().expect("id").to_owned()
        }
    }
}

/// The outcome of a legacy read answered with `res`: 200 with the HTML password form is upstream's
/// answer to a missing password, any other 200 is the paste, whose text `text_of` finds in the
/// body.
async fn legacy_outcome(res: reqwest::Response, text_of: impl Fn(String) -> String) -> Outcome {
    match res.status() {
        StatusCode::OK => {
            let body = res.text().await.expect("text");
            if body.contains(r#"type="password""#) {
                Outcome::PasswordRequired
            } else {
                Outcome::Text(text_of(body))
            }
        }
        StatusCode::FORBIDDEN => Outcome::WrongPassword,
        StatusCode::NOT_FOUND => Outcome::NotFound,
        status => unreachable!("legacy read answered {status}"),
    }
}

/// Read `id` through `route`. `text` is the paste's content, which the HTML route shows inside its
/// page: that route's outcome is the text when the page contains it.
async fn read_with(
    client: &Client,
    route: Read,
    id: &str,
    password: Option<&str>,
    text: &str,
) -> Outcome {
    match route {
        Read::Raw | Read::Download => {
            let path = match route {
                Read::Raw => "raw",
                _ => "dl",
            };
            let mut request = client.get(&format!("/{path}/{id}"));
            if let Some(password) = password {
                request = request.header(PASSWORD_HEADER_NAME, password);
            }
            let res = request.send().await.expect("raw or dl");
            legacy_outcome(res, |body| body).await
        }
        Read::Html => {
            let mut form = vec![("confirm_burn", "1")];
            if let Some(password) = password {
                form.push(("password", password));
            }
            let res = client
                .post(&format!("/{id}"))
                .form(&form)
                .send()
                .await
                .expect("html");
            legacy_outcome(res, |body| {
                if body.contains(text) {
                    text.to_owned()
                } else {
                    body
                }
            })
            .await
        }
        Read::V1 => {
            let mut body = json!({"confirm_burn": true});
            if let Some(password) = password {
                body["password"] = json!(password);
            }
            let res = call(
                client.post(&format!("/v1/pastes/{id}/content")).json(&body),
                "post",
                "/v1/pastes/{id}/content",
            )
            .await;
            match (res.status, res.error_type().as_str()) {
                (StatusCode::OK, _) => Outcome::Text(
                    res.json()["content"]["text"]
                        .as_str()
                        .expect("text")
                        .to_owned(),
                ),
                (StatusCode::CONFLICT, "password_required") => Outcome::PasswordRequired,
                (StatusCode::UNAUTHORIZED, _) => Outcome::WrongPassword,
                (StatusCode::NOT_FOUND, _) => Outcome::NotFound,
                (status, kind) => unreachable!("/v1 answered {status} {kind}"),
            }
        }
    }
}

/// A read with this password, and the outcome it must have.
type Step = (Option<&'static str>, Outcome);

/// Each scenario: the paste, then its reads in order.
fn scenarios() -> Vec<(Paste, Vec<Step>)> {
    let text = |text: &str| Outcome::Text(text.to_owned());
    vec![
        (
            Paste {
                text: "plain",
                password: None,
                burn: false,
            },
            vec![(None, text("plain")), (None, text("plain"))],
        ),
        (
            Paste {
                text: "locked",
                password: Some("right"),
                burn: false,
            },
            vec![
                (None, Outcome::PasswordRequired),
                (Some("wrong"), Outcome::WrongPassword),
                (Some("right"), text("locked")),
            ],
        ),
        (
            Paste {
                text: "once",
                password: None,
                burn: true,
            },
            vec![(None, text("once")), (None, Outcome::NotFound)],
        ),
        (
            Paste {
                text: "both",
                password: Some("right"),
                burn: true,
            },
            vec![
                (Some("wrong"), Outcome::WrongPassword),
                (Some("right"), text("both")),
                (Some("right"), Outcome::NotFound),
            ],
        ),
    ]
}

fn settings() -> Settings {
    Settings {
        rate: RateLimit::per_minute(NonZeroU32::new(1000).expect("non-zero")),
        ..Settings::default()
    }
}

/// Run every scenario, creating through `creator` and reading through `reader`.
async fn scenarios_end_alike(creator: Create, reader: Read) {
    let client = Client::with_settings(StoreCookies(false), settings()).await;

    for (paste, reads) in scenarios() {
        let id = create_with(&client, creator, &paste).await;
        for (step, (password, expected)) in reads.into_iter().enumerate() {
            let outcome = read_with(&client, reader, &id, password, paste.text).await;
            assert_eq!(
                outcome, expected,
                "{creator:?} then {reader:?}, {:?} step {step}",
                paste.text
            );
        }
    }
}

// Every create route and every read route, including both crossings, one test per pair so their
// key derivations run in parallel. The HTML page and `/dl` read through their own code, so each
// gets a pair; the other crossings add derivations without another path.
#[tokio::test]
async fn legacy_create_then_raw() {
    scenarios_end_alike(Create::Legacy, Read::Raw).await;
}

#[tokio::test]
async fn form_create_then_v1_read() {
    scenarios_end_alike(Create::Form, Read::V1).await;
}

#[tokio::test]
async fn v1_create_then_raw() {
    scenarios_end_alike(Create::V1, Read::Raw).await;
}

#[tokio::test]
async fn v1_create_then_v1_read() {
    scenarios_end_alike(Create::V1, Read::V1).await;
}

#[tokio::test]
async fn v1_create_then_html() {
    scenarios_end_alike(Create::V1, Read::Html).await;
}

#[tokio::test]
async fn form_create_then_download() {
    scenarios_end_alike(Create::Form, Read::Download).await;
}

#[tokio::test]
async fn read_routes_consume_a_burn_paste_once_between_them() {
    let client = Client::with_settings(StoreCookies(false), settings()).await;
    let paste = Paste {
        text: "once",
        password: None,
        burn: true,
    };
    let id = create_with(&client, Create::Legacy, &paste).await;

    // Every read route races for one paste, twice each; exactly one reader gets it.
    let reader = |route| read_with(&client, route, &id, None, paste.text);
    let (raw, download, html, v1, raw2, download2, html2, v12) = tokio::join!(
        reader(Read::Raw),
        reader(Read::Download),
        reader(Read::Html),
        reader(Read::V1),
        reader(Read::Raw),
        reader(Read::Download),
        reader(Read::Html),
        reader(Read::V1),
    );
    let outcomes = [raw, download, html, v1, raw2, download2, html2, v12];
    let read = outcomes
        .iter()
        .filter(|outcome| **outcome == Outcome::Text(String::from("once")))
        .count();
    let gone = outcomes
        .iter()
        .filter(|outcome| **outcome == Outcome::NotFound)
        .count();
    assert_eq!((read, gone), (1, 7), "{outcomes:?}");
}

#[tokio::test]
async fn legacy_routes_answer_the_client_rate_with_its_retry_after() {
    let client = Client::with_settings(
        StoreCookies(false),
        Settings {
            rate: RateLimit::per_minute(NonZeroU32::new(2).expect("non-zero")),
            ..Settings::default()
        },
    )
    .await;
    let paste = Paste {
        text: "x",
        password: Some("pw"),
        burn: false,
    };
    // The create spends one token, the wrong password the other.
    let id = create_with(&client, Create::Legacy, &paste).await;
    assert_eq!(
        read_with(&client, Read::Raw, &id, Some("wrong"), paste.text).await,
        Outcome::WrongPassword
    );

    let raw = client
        .get(&format!("/raw/{id}"))
        .header(PASSWORD_HEADER_NAME, "pw")
        .send()
        .await
        .expect("raw");
    let create = client
        .post_json()
        .json(&json!({"text": "y", "password": "pw"}))
        .send()
        .await
        .expect("legacy create");
    let form = client
        .post(&format!("/{id}"))
        .form(&[("password", "pw")])
        .send()
        .await
        .expect("password form");
    for res in [raw, create, form] {
        assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
        let retry_after = res
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .expect("Retry-After");
        assert!((29..=30).contains(&retry_after), "{retry_after}");
    }
}

#[tokio::test]
async fn html_password_on_a_plain_paste_spends_no_derivation() {
    // Two tokens: the encrypted create spends one, the first encrypted read the other.
    let client = Client::with_settings(
        StoreCookies(false),
        Settings {
            rate: RateLimit::per_minute(NonZeroU32::new(2).expect("non-zero")),
            ..Settings::default()
        },
    )
    .await;
    let plain = Paste {
        text: "plain",
        password: None,
        burn: false,
    };
    let plain = create_with(&client, Create::Legacy, &plain).await;
    let locked = Paste {
        text: "locked",
        password: Some("pw"),
        burn: false,
    };
    let locked = create_with(&client, Create::Legacy, &locked).await;

    let post = |path: String, password: &'static str| {
        client.post(&path).form(&[("password", password)]).send()
    };
    for _ in 0..4 {
        let res = post(format!("/{plain}"), "x").await.expect("plain");
        assert_eq!(res.status(), StatusCode::OK);
    }

    // The plain reads left the token, and an encrypted read still spends it.
    let res = post(format!("/{locked}"), "pw").await.expect("locked");
    assert_eq!(res.status(), StatusCode::OK);
    let res = post(format!("/{locked}"), "pw").await.expect("locked");
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
}
