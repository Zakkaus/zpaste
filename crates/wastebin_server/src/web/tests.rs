//! The reading page and the web app's files, through the router (docs/architecture.md, "Serving
//! a page"; invariants 3, 4, 5, 16 and 17).

use reqwest::{StatusCode, header};
use serde_json::{Value, json};

use super::{WEB, dist, mime_of};
use crate::test_helpers::{Client, StoreCookies};

const TEXT: &str = "# Plan\n\nFirst </script><script>alert(1)</script> & more\nSecond line\n";

/// Create a paste through `/v1` without credentials; returns its id and owner token.
async fn create(client: &Client, paste: Value) -> (String, String) {
    let res = client
        .post("/v1/pastes")
        .json(&json!({ "paste": paste }))
        .send()
        .await
        .expect("create");
    assert_eq!(res.status(), StatusCode::CREATED);
    let body: Value = res.json().await.expect("json");
    (
        body["paste"]["id"].as_str().expect("id").to_owned(),
        body["paste"]["owner_token"]
            .as_str()
            .expect("owner token")
            .to_owned(),
    )
}

/// The paste the page embeds, parsed as the app parses it.
fn embedded(page: &str) -> Option<Value> {
    let start = page.find(r#"<script type="application/json" id="zpaste-paste">"#)?;
    let json = &page[start..];
    let json = &json[json.find('>')? + 1..json.find("</script>")?];
    Some(serde_json::from_str(json).expect("the embedded paste parses"))
}

/// The `content` of the `<meta>` whose `property` or `name` is `key`.
fn meta<'a>(page: &'a str, key: &str) -> Option<&'a str> {
    let at = page
        .find(&format!("property=\"{key}\""))
        .or_else(|| page.find(&format!("name=\"{key}\"")))?;
    let rest = &page[at..];
    let rest = &rest[rest.find("content=\"")? + 9..];
    Some(&rest[..rest.find('"')?])
}

async fn page(client: &Client, path: &str, authorization: Option<&str>) -> reqwest::Response {
    let mut request = client.get(path);
    if let Some(token) = authorization {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    request.send().await.expect("page")
}

#[test]
fn reading_policy_admits_the_build_styles_and_no_inline() {
    let csp = WEB.csp.to_str().unwrap();
    for source in dist::STYLE_SRC {
        assert!(csp.contains(source), "{csp}");
    }
    assert!(csp.contains("img-src 'self' https: data:;"), "{csp}");
    assert!(csp.contains("connect-src 'self';"), "{csp}");
    assert!(csp.contains("script-src 'self';"), "{csp}");
    assert!(!csp.contains("unsafe"), "{csp}");
}

#[test]
fn file_types() {
    assert_eq!(mime_of("/assets/index-x.js").0, "text/javascript");
    assert_eq!(mime_of("/theme-boot.js").0, "text/javascript");
    assert!(!mime_of("/assets/figtree-latin-400-normal-x.woff2").1);
    assert_eq!(mime_of("/LICENSE").0, "application/octet-stream");
}

#[tokio::test]
async fn plain_paste_is_embedded_with_preview_and_noscript_copy() {
    let client = Client::new(StoreCookies(false)).await;
    let (id, _) = create(&client, json!({"content": TEXT, "extension": "md"})).await;

    let res = page(&client, &format!("/{id}.md"), None).await;
    assert_eq!(res.status(), StatusCode::OK);
    let headers = res.headers().clone();
    assert_eq!(
        headers[header::CONTENT_SECURITY_POLICY],
        WEB.csp.to_str().unwrap()
    );
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    assert!(
        headers[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    let body = res.text().await.unwrap();

    // Invariant 5: the paste cannot end its element or start a script of its own.
    assert_eq!(body.matches("zpaste-paste").count(), 1, "{body}");
    assert!(!body.contains("<script>alert"), "{body}");
    let data = embedded(&body).expect("embedded paste");
    assert_eq!(data["content"], TEXT);
    assert_eq!(data["paste"]["id"], id.as_str());
    assert_eq!(data["paste"]["extension"], "md");
    assert_eq!(data["paste"]["title"], Value::Null);
    assert_eq!(data["paste"]["encrypted"], false);
    assert_eq!(data["paste"]["burn_after_reading"], false);
    assert_eq!(data["paste"]["version"], 1);
    assert!(data["paste"]["created_at"].as_str().is_some(), "{data}");
    let version_id = data["paste"]["version_id"].as_str().unwrap();
    assert_eq!(data["etag"], format!("\"{version_id}\""));
    assert_eq!(data["url_extension"], "md");
    assert_eq!(data["can_edit"], false);

    // The link preview: the first line as the title, the next lines as the summary.
    assert_eq!(meta(&body, "og:title"), Some("Plan"));
    assert_eq!(meta(&body, "twitter:title"), Some("Plan"));
    let summary =
        "First &lt;/script&gt;&lt;script&gt;alert(1)&lt;/script&gt; &amp; more\nSecond line";
    assert_eq!(meta(&body, "og:description"), Some(summary));
    assert_eq!(meta(&body, "description"), Some(summary));
    assert_eq!(
        meta(&body, "og:url"),
        Some(format!("https://localhost:8888/{id}.md").as_str())
    );
    assert!(body.contains("<title>Plan · test</title>"), "{body}");

    // Without JavaScript the text is still there, escaped, and wraps.
    assert!(
        body.contains("<noscript><pre># Plan\n\nFirst &lt;/script&gt;&lt;script&gt;alert(1)"),
        "{body}"
    );
    assert!(body.contains(WEB.noscript_css.route()), "{body}");

    // Without an extension in the URL the app picks the stored one.
    let body = page(&client, &format!("/{id}"), None)
        .await
        .text()
        .await
        .unwrap();
    assert_eq!(embedded(&body).unwrap()["url_extension"], Value::Null);
}

#[tokio::test]
async fn titled_paste_previews_its_title() {
    let client = Client::new(StoreCookies(false)).await;
    let (id, _) = create(
        &client,
        json!({"content": "one\ntwo", "title": "Notes <1>"}),
    )
    .await;

    let body = page(&client, &format!("/{id}"), None)
        .await
        .text()
        .await
        .unwrap();
    assert_eq!(meta(&body, "og:title"), Some("Notes &lt;1&gt;"));
    assert_eq!(meta(&body, "og:description"), Some("one\ntwo"));
    assert_eq!(embedded(&body).unwrap()["paste"]["title"], "Notes <1>");
}

/// Until the web app has its own screens for them, a sealed paste's page is upstream's form
/// (open item A5).
#[tokio::test]
async fn sealed_pastes_get_their_form_and_are_not_consumed() {
    let client = Client::new(StoreCookies(false)).await;
    let (burn, _) = create(
        &client,
        json!({"content": "burn-body-xyz", "title": "burn-title-xyz", "burn_after_reading": true}),
    )
    .await;
    let (locked, _) = create(
        &client,
        json!({"content": "locked-body-xyz", "title": "locked-title-xyz", "password": "pw"}),
    )
    .await;
    let (both, _) = create(
        &client,
        json!({"content": "both-body-xyz", "title": "both-title-xyz", "password": "pw",
               "burn_after_reading": true}),
    )
    .await;

    for (id, confirm) in [
        (&burn, true),
        (&locked, false),
        (&both, true),
        (&burn, true),
    ] {
        let res = page(&client, &format!("/{id}"), None).await;
        assert_eq!(res.status(), StatusCode::OK);
        let cache_control = res.headers().get(header::CACHE_CONTROL).cloned();
        let body = res.text().await.unwrap();
        assert!(body.contains(&format!("action=\"/{id}\"")), "{body}");
        if confirm {
            assert!(body.contains("name=\"confirm_burn\" value=\"1\""), "{body}");
            assert!(!body.contains("name=\"password\""), "{body}");
        } else {
            assert!(body.contains("name=\"password\""), "{body}");
            assert_eq!(cache_control.unwrap(), "no-store");
        }
        // Invariant 4: no content, no title and no summary.
        assert!(!body.contains("-xyz"), "{body}");
        assert!(embedded(&body).is_none(), "{body}");
        assert!(meta(&body, "og:description").is_none(), "{body}");
    }

    // Invariant 3: the burn paste is still there until the reader confirms.
    let res = client
        .post(&format!("/v1/pastes/{burn}/content"))
        .json(&json!({"confirm_burn": true}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["content"]["text"], "burn-body-xyz");
    assert_eq!(
        page(&client, &format!("/{burn}"), None).await.status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn owner_credentials_show_edit() {
    let client = Client::new(StoreCookies(false)).await;
    let (id, token) = create(&client, json!({"content": "mine"})).await;
    let (_, other) = create(&client, json!({"content": "theirs"})).await;
    let path = format!("/{id}");

    let can_edit = |authorization: Option<&'static str>, token: String| {
        let client = &client;
        let path = &path;
        async move {
            let res = match authorization {
                Some("token") => page(client, path, Some(&token)).await,
                Some(cookie) => client
                    .get(path)
                    .header(header::COOKIE, cookie)
                    .send()
                    .await
                    .unwrap(),
                None => page(client, path, None).await,
            };
            assert_eq!(res.status(), StatusCode::OK);
            embedded(&res.text().await.unwrap()).unwrap()["can_edit"].clone()
        }
    };

    assert_eq!(can_edit(Some("token"), token.clone()).await, true);
    assert_eq!(can_edit(Some("token"), other).await, false);
    // A token that fails verification only hides Edit; the page still opens.
    assert_eq!(can_edit(Some("token"), String::from("forged")).await, false);
    assert_eq!(can_edit(None, String::new()).await, false);
    assert_eq!(can_edit(Some("uid=forged"), String::new()).await, false);

    // The cookie that `?owner=` sets shows Edit as well.
    let res = client
        .get(&path)
        .query(&[("owner", &token)])
        .send()
        .await
        .unwrap();
    let cookie = res.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let res = client
        .get(&path)
        .header(header::COOKIE, cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(
        embedded(&res.text().await.unwrap()).unwrap()["can_edit"],
        true
    );
}

#[tokio::test]
async fn owner_exchange_leaves_the_url_before_any_html() {
    let client = Client::new(StoreCookies(false)).await;
    let (id, token) = create(&client, json!({"content": "x", "extension": "rs"})).await;

    for (path, owner, location, cookie) in [
        (
            format!("/{id}.rs"),
            token.as_str(),
            format!("/{id}.rs"),
            true,
        ),
        (format!("/{id}.rs"), "forged", format!("/{id}.rs"), false),
        (
            String::from("/not-an-id"),
            token.as_str(),
            String::from("/"),
            false,
        ),
    ] {
        let res = client
            .get(&path)
            .query(&[("owner", owner)])
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FOUND, "{path}");
        assert_eq!(res.headers()[header::LOCATION], location.as_str(), "{path}");
        assert_eq!(res.headers()[header::CACHE_CONTROL], "no-store", "{path}");
        assert_eq!(
            res.headers().contains_key(header::SET_COOKIE),
            cookie,
            "{path}"
        );
        assert!(res.text().await.unwrap().is_empty(), "{path}");
    }
}

#[tokio::test]
async fn missing_pastes_get_the_page_with_404() {
    let client = Client::new(StoreCookies(false)).await;

    for path in ["/aaaaaa", "/aaaaaa.md", "/not-an-id"] {
        let res = page(&client, path, None).await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND, "{path}");
        assert_eq!(
            res.headers()[header::CONTENT_SECURITY_POLICY],
            WEB.csp.to_str().unwrap()
        );
        let body = res.text().await.unwrap();
        assert!(body.contains("<div id=\"root\"></div>"), "{body}");
        assert!(embedded(&body).is_none(), "{body}");
    }
}

#[tokio::test]
async fn old_rendered_view_redirects_to_the_reading_page() {
    let client = Client::new(StoreCookies(false)).await;
    let (id, _) = create(&client, json!({"content": "# x", "extension": "md"})).await;

    let res = client.get(&format!("/md/{id}.md")).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::FOUND);
    assert_eq!(
        res.headers()[header::LOCATION],
        format!("/{id}.md").as_str()
    );
    let res = client.get(&format!("/md/{id}")).send().await.unwrap();
    assert_eq!(res.headers()[header::LOCATION], format!("/{id}").as_str());
    let res = client.get("/md/not-an-id").send().await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn web_files_are_served_with_their_cache_policy() {
    let client = Client::new(StoreCookies(false)).await;

    for asset in WEB.files.iter().chain([&WEB.noscript_css]) {
        let res = client.get(asset.route()).send().await.unwrap();
        assert_eq!(res.status(), StatusCode::OK, "{}", asset.route());
        let cache_control = res.headers()[header::CACHE_CONTROL]
            .to_str()
            .unwrap()
            .to_owned();
        let hashed =
            asset.route().starts_with("/assets/") || std::ptr::eq(asset, &WEB.noscript_css);
        assert_eq!(
            cache_control.contains("immutable"),
            hashed,
            "{} {cache_control}",
            asset.route()
        );
        assert_eq!(
            cache_control.contains("no-cache"),
            !hashed,
            "{cache_control}"
        );
    }
}
