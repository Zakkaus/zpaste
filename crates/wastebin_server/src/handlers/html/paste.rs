use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Form, Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::SignedCookieJar;
use serde::Deserialize;

use crate::cache::{Key, Mode};
use crate::client::{ClientKey, RateLimit};
use crate::handlers::extract::{Theme, Uids, verify_owner_token};
use crate::handlers::html::{BurnConfirmation, FormAction, PasswordInput, make_error_response};
use crate::handlers::{NoStore, uid_cookie};
use crate::i18n::Lang;
use crate::service::{self, Failure};
use crate::v1::auth;
use crate::web::WEB;
use crate::web::shell::{self, Embedded, PasteMeta, Reading};
use crate::{AppState, Cache, Database, Highlighter, Page};
use wastebin_core::crypto::Password;
use wastebin_core::db;
use wastebin_core::db::read::{Data, Entry, Metadata};
use wastebin_core::expiration::Expiration;

/// Magic-link handoff: when a paste was created via the JSON API, the response contains a signed
/// `owner` token. Opening `/<id>?owner=<token>` lets the browser claim ownership of the paste, the
/// server validates the token, appends the uid to the user's signed `uid` cookie, and redirects to
/// the clean paste URL so the token never leaks via Referer.
#[derive(Deserialize, Debug)]
pub(crate) struct OwnerHandoff {
    pub(crate) owner: Option<String>,
}

/// The reading page `/<id>[.<ext>]`: the web app's HTML shell (docs/architecture.md, "Serving a
/// page"). It never consumes a paste and never derives a key; an encrypted or burn-after-reading
/// paste's page carries nothing of it.
pub async fn get(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(handoff): Query<OwnerHandoff>,
    jar: SignedCookieJar,
    uids: Option<Uids>,
    headers: HeaderMap,
) -> Response {
    let key = path.parse::<Key>().ok();

    // Invariant 16: the token leaves the URL before any HTML is produced.
    if let Some(token) = handoff.owner.as_deref() {
        let location = key
            .as_ref()
            .map_or_else(|| String::from("/"), |key| format!("/{key}"));
        let redirect = found(&location).no_store();
        return match (key, verify_owner_token(&state.key, token)) {
            (Some(_), Some(claimed_uid)) => {
                let mut new_uids = uids.map(|Uids(list)| list).unwrap_or_default();
                if !new_uids.contains(&claimed_uid) {
                    new_uids.push(claimed_uid);
                }
                (jar.add(uid_cookie(&new_uids)), redirect).into_response()
            }
            _ => redirect,
        };
    }

    let Some(key) = key else {
        return reading_page(&state, StatusCode::NOT_FOUND, &Reading::Missing, &path);
    };
    let (head, text) = match service::peek(&state, key.id).await {
        Ok(read) => read,
        Err(Failure::NotFound) => {
            return reading_page(&state, StatusCode::NOT_FOUND, &Reading::Missing, &path);
        }
        Err(err) => {
            tracing::error!(%err, "reading page");
            return reading_page(
                &state,
                StatusCode::INTERNAL_SERVER_ERROR,
                &Reading::Missing,
                &path,
            );
        }
    };
    let Some(text) = text else {
        return reading_page(&state, StatusCode::OK, &Reading::Sealed, &path);
    };

    // The owner's credentials only show the Edit action; every write is checked on its own. A
    // Bearer token that fails, like no credentials, shows the page without it.
    let can_edit = auth::caller(&headers, &state.key, &state.page.base_url, false)
        .is_ok_and(|caller| head.uid.is_some_and(|uid| caller.uids().contains(&uid)));
    let embedded = Embedded {
        paste: PasteMeta::new(&head),
        content: &text,
        etag: format!("\"{}\"", head.version_id),
        url_extension: key.ext.as_deref().filter(|ext| !ext.is_empty()),
        can_edit,
    };
    reading_page(&state, StatusCode::OK, &Reading::Plain(embedded), &path)
}

/// `GET /md/<id>[.<ext>]`, upstream's rendered Markdown view: the reading page renders Markdown
/// now, so the old address leads there.
pub async fn rendered(Path(path): Path<String>) -> Response {
    match path.parse::<Key>() {
        Ok(key) => found(&format!("/{key}")),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// 302 to `location`, which the caller built from a parsed [`Key`] and so is a valid header.
fn found(location: &str) -> Response {
    let location = HeaderValue::try_from(location).unwrap_or(HeaderValue::from_static("/"));
    (StatusCode::FOUND, [(header::LOCATION, location)]).into_response()
}

/// The shell for `reading` at `/<path>`. Its policy admits the web app's run-time styles and the
/// images a paste links (invariant 17). The page can depend on the reader's cookie (`can_edit`),
/// so no shared cache keeps it and the reader's own revalidates it.
fn reading_page(
    state: &AppState,
    status: StatusCode,
    reading: &Reading<'_>,
    path: &str,
) -> Response {
    let web = &*WEB;
    let url = format!(
        "{}/{}",
        state.page.base_url.as_str().trim_end_matches('/'),
        path
    );
    let html = shell::render(
        &web.template,
        reading,
        &state.page.title,
        &url,
        web.noscript_css.route(),
    );

    (
        status,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            ),
            (header::CONTENT_SECURITY_POLICY, web.csp.clone()),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("private, no-cache"),
            ),
        ],
        html,
    )
        .into_response()
}

#[derive(Deserialize, Debug)]
pub(crate) struct PasteForm {
    #[serde(default)]
    pub(crate) password: Option<String>,
    #[serde(default)]
    pub(crate) confirm_burn: Option<String>,
}

/// Paste view showing the formatted paste.
#[derive(Template, WebTemplate)]
#[template(path = "formatted.html")]
pub(crate) struct Paste {
    page: Page,
    key: Key,
    theme: Option<Theme>,
    lang: Lang,
    can_delete: bool,
    /// If the paste still in the database and can be fetched with another request.
    is_available: bool,
    /// Expiration in case it was set.
    expiration: Option<Expiration>,
    html: String,
    title: Option<String>,
    /// Whether the paste's extension identifies it as Markdown, enabling the rendered-view toggle.
    is_markdown: bool,
}

/// Return `true` if `ext` identifies a Markdown paste.
pub(crate) fn is_markdown_ext(ext: Option<&str>) -> bool {
    ext.is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
}

/// `POST /<id>`: upstream's password and burn-confirmation forms, which post back to the paste
/// (docs/architecture.md, "Legacy routes", open item A5).
#[expect(clippy::too_many_arguments)]
pub async fn post<E>(
    State(cache): State<Cache>,
    State(page): State<Page>,
    State(db): State<Database>,
    State(rate): State<RateLimit>,
    client: ClientKey,
    State(highlighter): State<Highlighter>,
    Path(id): Path<String>,
    uids: Option<Uids>,
    theme: Option<Theme>,
    lang: Lang,
    form: Result<Form<PasteForm>, E>,
) -> Response {
    async {
        let form = form.ok().map(|Form(form)| form);
        let password = form
            .as_ref()
            .and_then(|form| form.password.as_ref())
            .filter(|password| !password.is_empty())
            .map(|password| Password::from(password.as_bytes().to_vec()));
        let confirmed = form.as_ref().and_then(|form| form.confirm_burn.as_deref()) == Some("1");
        let no_password = password.is_none();
        let key: Key = id.parse()?;
        let metadata = db.get_metadata(key.id).await?;

        if metadata.must_be_deleted && !confirmed {
            return Ok(BurnConfirmation {
                page: page.clone(),
                theme: theme.clone(),
                lang,
                title: metadata.title.clone(),
                action: FormAction::source(&key),
            }
            .into_response());
        }

        // A password derives a key only for an encrypted paste; a plain one ignores it.
        if password.is_some() && metadata.encrypted {
            service::admit(&rate, client)?;
        }

        let (data, is_available, encrypted) = match db.get(key.id, password).await {
            Ok(Entry::Regular(data)) => (data, true, false),
            Ok(Entry::Burned(data)) => (data, false, false),
            Ok(Entry::Encrypted(data)) => {
                let is_available = !data.metadata.must_be_deleted;
                (data, is_available, true)
            }
            Err(db::Error::NoPassword) => {
                return Ok(PasswordInput {
                    page: page.clone(),
                    theme: theme.clone(),
                    lang,
                    action: FormAction::source(&key),
                }
                .into_response()
                .no_store());
            }
            Err(err) => return Err(err.into()),
        };

        let Data { text, metadata } = data;
        let Metadata {
            uid: owner_uid,
            title,
            expiration,
            ..
        } = metadata;

        let can_delete = match (uids, owner_uid) {
            (Some(Uids(uids)), Some(owner_uid)) => uids.contains(&owner_uid),
            _ => false,
        };

        let html = if let Some(html) = cache.get(&key, Mode::Source) {
            tracing::trace!(?key, "found cached item");
            html.into_inner()
        } else {
            let ext = key.ext.clone();
            let highlighter = highlighter.clone();
            let html =
                tokio::task::spawn_blocking(move || highlighter.highlight(text, ext)).await??;

            if is_available && no_password {
                tracing::trace!(?key, "cache item");
                cache.put(&key, Mode::Source, html.clone());
            }

            html.into_inner()
        };

        let is_markdown = is_markdown_ext(key.ext.as_deref());
        let paste = Paste {
            page: page.clone(),
            key,
            theme: theme.clone(),
            lang,
            can_delete,
            is_available,
            expiration,
            html,
            title,
            is_markdown,
        };

        let response = paste.into_response();
        Ok(if encrypted {
            response.no_store()
        } else {
            response
        })
    }
    .await
    .unwrap_or_else(|err: crate::Error| make_error_response(err, page, theme, lang))
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::{Client, StoreCookies};
    use reqwest::StatusCode;

    #[tokio::test]
    async fn unknown_paste() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(false)).await;

        let res = client.get("/000000").send().await?;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        Ok(())
    }

    #[tokio::test]
    async fn password_prompt_keeps_extension() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(false)).await;
        let data = crate::handlers::insert::form::Entry {
            text: String::from("fn main() {}"),
            extension: Some(String::from("rs")),
            password: String::from("hunter2"),
            ..Default::default()
        };

        let res = client.post_form().form(&data).send().await?;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        let location = res.headers().get("location").unwrap().to_str()?.to_owned();
        assert!(location.ends_with(".rs"), "location: {location}");

        // The prompt submits back to the very same paste, extension included.
        let body = client.post(&location).send().await?.text().await?;
        assert!(
            body.contains(&format!("action=\"{location}\"")),
            "body: {body}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn encrypted_paste_is_not_stored() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(false)).await;
        let password = "hunter2";
        let data = crate::handlers::insert::form::Entry {
            text: String::from("secret-body-xyz"),
            password: password.to_string(),
            ..Default::default()
        };

        let res = client.post_form().form(&data).send().await?;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        let location = res.headers().get("location").unwrap().to_str()?.to_owned();

        // The password prompt is not cacheable.
        let res = client.post(&location).send().await?;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers().get(reqwest::header::CACHE_CONTROL).unwrap(),
            "no-store"
        );

        // The decrypted paste is not cacheable either.
        let res = client
            .post(&location)
            .form(&[("password", password)])
            .header(reqwest::header::ACCEPT, "text/html; charset=utf-8")
            .send()
            .await?;
        assert_eq!(res.status(), StatusCode::OK);
        let cache_control = res.headers().get(reqwest::header::CACHE_CONTROL).cloned();
        let body = res.text().await?;
        assert!(body.contains("secret-body-xyz"), "body: {body}");
        assert_eq!(cache_control.unwrap(), "no-store");

        Ok(())
    }
}
