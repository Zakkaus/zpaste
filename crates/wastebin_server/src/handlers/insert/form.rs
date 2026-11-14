use std::num::NonZeroU32;

use axum::extract::{Form, State};
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::SignedCookieJar;
use serde::{Deserialize, Serialize};

use crate::cache::valid_extension;
use crate::client::ClientKey;
use crate::handlers::extract::{Theme, Uids};
use crate::handlers::html::make_error_response;
use crate::handlers::uid_cookie;
use crate::i18n::Lang;
use crate::service;
use crate::{AppState, Page};
use wastebin_core::db::write;
use wastebin_core::id;

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct Entry {
    pub text: String,
    pub extension: Option<String>,
    pub expires: Option<String>,
    pub password: String,
    pub title: String,
    #[serde(rename = "burn-after-reading")]
    pub burn_after_reading: Option<String>,
}

impl From<Entry> for write::Entry {
    fn from(entry: Entry) -> Self {
        let burn_after_reading = entry.burn_after_reading.map(|s| s == "on");
        let password = (!entry.password.is_empty()).then_some(entry.password);
        let title = (!entry.title.is_empty()).then_some(entry.title);
        let expires = entry
            .expires
            .and_then(|expires| expires.parse::<NonZeroU32>().ok());

        Self {
            text: entry.text,
            extension: entry.extension,
            expires,
            burn_after_reading,
            uid: None,
            password,
            title,
        }
    }
}

/// The form create, an adapter over [`service::create`].
#[allow(
    clippy::too_many_arguments,
    reason = "one extractor per request part, as axum takes them"
)]
pub async fn post<E: std::fmt::Debug>(
    State(page): State<Page>,
    State(state): State<AppState>,
    client: ClientKey,
    jar: SignedCookieJar,
    uids: Option<Uids>,
    theme: Option<Theme>,
    lang: Lang,
    entry: Result<Form<Entry>, E>,
) -> Response {
    let Ok(Form(entry)) = entry else {
        return make_error_response(crate::Error::MalformedForm, page, theme, lang);
    };

    if let Some(extension) = &entry.extension
        && !valid_extension(extension)
    {
        return make_error_response(
            crate::Error::Id(id::Error::IllegalCharacters),
            page,
            theme,
            lang,
        );
    }

    let entry: write::Entry = entry.into();
    let mut uids = uids.map(|Uids(uids)| uids).unwrap_or_default();

    async {
        // Create as the existing primary uid (first in the cookie list) or a new one. Re-set the
        // cookie with the full list unchanged so claimed uids survive.
        let (head, uid) = service::create(&state, client, entry, uids.first().copied()).await?;
        if uids.is_empty() {
            uids.push(uid);
        }

        let url_path = match &head.extension {
            Some(extension) => format!("{}.{extension}", head.id),
            None => head.id.to_string(),
        };
        let url = if head.burn_after_reading {
            format!("/burn/{url_path}")
        } else {
            format!("/{url_path}")
        };

        let cookie = uid_cookie(&uids);

        Ok((jar.add(cookie), Redirect::to(&url)).into_response())
    }
    .await
    .unwrap_or_else(|err: crate::Error| make_error_response(err, page, theme, lang))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{Client, StoreCookies};
    use reqwest::{StatusCode, header};
    use std::collections::HashMap;

    #[tokio::test]
    async fn insert() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(false)).await;
        let data = Entry {
            text: String::from("FooBarBaz"),
            ..Default::default()
        };

        let res = client.post_form().form(&data).send().await?;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);

        let location = res.headers().get("location").unwrap().to_str()?;

        let res = client
            .get(location)
            .header(header::ACCEPT, "text/html; charset=utf-8")
            .send()
            .await?;

        assert_eq!(res.status(), StatusCode::OK);

        let header = res.headers().get(header::CONTENT_TYPE).unwrap();
        assert!(header.to_str().unwrap().contains("text/html"));

        let content = res.text().await?;
        assert!(content.contains("FooBarBaz"));

        let res = client
            .get(&format!("/raw{location}"))
            .header(header::ACCEPT, "text/html; charset=utf-8")
            .send()
            .await?;

        assert_eq!(res.status(), StatusCode::OK);

        let header = res.headers().get(header::CONTENT_TYPE).unwrap();
        assert!(header.to_str().unwrap().contains("text/plain"));

        let content = res.text().await?;
        assert_eq!(content, "FooBarBaz");

        Ok(())
    }

    #[tokio::test]
    async fn insert_rejects_invalid_extension() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(false)).await;
        let data = Entry {
            text: String::from("FooBarBaz"),
            extension: Some(String::from("\"><img src=x>")),
            ..Default::default()
        };

        let res = client.post_form().form(&data).send().await?;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }

    #[tokio::test]
    async fn insert_accepts_dotted_extension() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(false)).await;
        let data = Entry {
            text: String::from("FooBarBaz"),
            // One of the picker's dotted extensions.
            extension: Some(String::from("CMakeLists.txt")),
            ..Default::default()
        };

        let res = client.post_form().form(&data).send().await?;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        let location = res.headers().get("location").unwrap().to_str()?;
        assert!(location.ends_with(".CMakeLists.txt"));

        let res = client
            .get(location)
            .header(header::ACCEPT, "text/html; charset=utf-8")
            .send()
            .await?;
        assert_eq!(res.status(), StatusCode::OK);

        Ok(())
    }

    #[tokio::test]
    async fn insert_fail() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(false)).await;

        let mut data = HashMap::new();
        data.insert("Hello", "World");

        let res = client.post_form().form(&data).send().await?;
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

        Ok(())
    }

    #[tokio::test]
    async fn insert_sets_uid_cookie() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(true)).await;
        let res = client.post_form().form(&Entry::default()).send().await?;
        let cookie = res.cookies().find(|cookie| cookie.name() == "uid").unwrap();
        assert_eq!(cookie.name(), "uid");
        assert!(cookie.value().len() > 40);
        assert_eq!(cookie.path().unwrap(), "/");
        assert!(cookie.http_only());
        assert!(cookie.same_site_lax());
        assert!(cookie.domain().is_none());
        assert!(cookie.expires().is_none());
        assert!(cookie.max_age().is_none());
        assert!(cookie.secure());

        Ok(())
    }
}
