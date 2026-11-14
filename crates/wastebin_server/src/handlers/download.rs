use std::fmt::Write;

use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum_extra::headers::HeaderValue;

use crate::cache::Key;
use crate::client::ClientKey;
use crate::handlers::NoStore;
use crate::handlers::extract::{Password, Theme};
use crate::handlers::html::{FormAction, PasswordInput, make_error_response};
use crate::i18n::Lang;
use crate::service::{self, Content, Failure};
use crate::{AppState, Page};

/// GET handler for downloading a paste, an adapter over [`service::read`] like `/raw`.
pub async fn get(
    Path(id): Path<String>,
    State(state): State<AppState>,
    State(page): State<Page>,
    client: ClientKey,
    theme: Option<Theme>,
    lang: Lang,
    password: Option<Password>,
) -> Response {
    async {
        let key: Key = id.parse()?;
        let password = password.map(|Password(password)| password);

        match service::read(&state, client, key.id, password, true).await {
            Ok(content) if content.head.encrypted => {
                Ok(get_download(&key, content).into_response().no_store())
            }
            Ok(content) => Ok(get_download(&key, content).into_response()),
            Err(Failure::PasswordRequired) => Ok(PasswordInput {
                page: page.clone(),
                theme: theme.clone(),
                lang,
                action: FormAction::source(&key),
            }
            .into_response()
            .no_store()),
            Err(err) => Err(err.into()),
        }
    }
    .await
    .unwrap_or_else(|err: crate::Error| make_error_response(err, page, theme, lang))
}

fn make_content_disposition(filename: &str) -> HeaderValue {
    let mut value = String::from("attachment; filename*=UTF-8''");

    for b in filename.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'~' | b'+') {
            value.push(b as char);
        } else {
            write!(value, "%{b:02X}").expect("writing to String");
        }
    }

    HeaderValue::try_from(value).unwrap_or_else(|_| HeaderValue::from_static("attachment"))
}

fn get_download(key: &Key, Content { head, text }: Content) -> impl IntoResponse {
    let filename = head.title.unwrap_or_else(|| key.to_string());

    let content_type = "text; charset=utf-8";
    let content_disposition = make_content_disposition(&filename);

    (
        [
            (header::CONTENT_TYPE, HeaderValue::from_static(content_type)),
            (header::CONTENT_DISPOSITION, content_disposition),
        ],
        text,
    )
}

#[cfg(test)]
mod tests {
    use crate::handlers::insert::form::Entry;
    use crate::test_helpers::{Client, StoreCookies};
    use http::header;
    use reqwest::StatusCode;

    #[tokio::test]
    async fn download() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(false)).await;
        let data = Entry {
            text: String::from("FooBarBaz"),
            ..Default::default()
        };

        let res = client.post_form().form(&data).send().await?;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);

        let location = res.headers().get("location").unwrap().to_str()?;
        let filename = &location[1..];
        let res = client.get(&format!("/dl/{filename}.cpp")).send().await?;
        assert_eq!(res.status(), StatusCode::OK);

        let content_disposition = res.headers().get(header::CONTENT_DISPOSITION).unwrap();
        assert_eq!(
            content_disposition.to_str()?,
            format!("attachment; filename*=UTF-8''{filename}.cpp"),
        );

        let content = res.text().await?;
        assert_eq!(content, "FooBarBaz");

        let res = client.get(&format!("/dl{location}")).send().await?;
        let content_disposition = res.headers().get(header::CONTENT_DISPOSITION).unwrap();
        assert_eq!(
            content_disposition.to_str()?,
            format!("attachment; filename*=UTF-8''{filename}"),
        );

        Ok(())
    }

    #[tokio::test]
    async fn download_title_with_quotes() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(false)).await;
        let data = Entry {
            text: String::from("content"),
            title: String::from(r#"file"name.txt"#),
            ..Default::default()
        };

        let res = client.post_form().form(&data).send().await?;
        let location = res.headers().get("location").unwrap().to_str()?;
        let res = client.get(&format!("/dl{location}")).send().await?;
        assert_eq!(res.status(), StatusCode::OK);

        let content_disposition = res.headers().get(header::CONTENT_DISPOSITION).unwrap();
        assert_eq!(
            content_disposition.to_str()?,
            "attachment; filename*=UTF-8''file%22name.txt",
        );

        Ok(())
    }

    #[tokio::test]
    async fn download_title_with_non_ascii() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(false)).await;
        let data = Entry {
            text: String::from("content"),
            title: String::from("café.txt"),
            ..Default::default()
        };

        let res = client.post_form().form(&data).send().await?;
        let location = res.headers().get("location").unwrap().to_str()?;
        let res = client.get(&format!("/dl{location}")).send().await?;
        assert_eq!(res.status(), StatusCode::OK);

        let content_disposition = res.headers().get(header::CONTENT_DISPOSITION).unwrap();
        assert_eq!(
            content_disposition.to_str()?,
            "attachment; filename*=UTF-8''caf%C3%A9.txt",
        );

        Ok(())
    }

    #[tokio::test]
    async fn encrypted_download_is_not_stored() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(false)).await;
        let password = "hunter2";
        let data = Entry {
            text: String::from("secret-body-xyz"),
            password: password.to_string(),
            ..Default::default()
        };

        let res = client.post_form().form(&data).send().await?;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        let location = res.headers().get("location").unwrap().to_str()?.to_owned();

        let res = client
            .get(&format!("/dl{location}"))
            .header(crate::handlers::extract::PASSWORD_HEADER_NAME, password)
            .send()
            .await?;
        assert_eq!(res.status(), StatusCode::OK);
        let cache_control = res.headers().get(header::CACHE_CONTROL).cloned();
        assert_eq!(res.text().await?, "secret-body-xyz");
        assert_eq!(cache_control.unwrap(), "no-store");

        Ok(())
    }
}
