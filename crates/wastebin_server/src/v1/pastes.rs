//! `/v1` handlers of this phase. Each `#[utoipa::path]` is the contract of its operation;
//! `docs/api/openapi.yaml` is generated from these annotations by the tests (see
//! `docs/api/README.md`).

use std::num::NonZeroU32;

use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::SignedCookieJar;
use serde::Deserialize;
#[cfg(test)]
use serde::Serialize;
#[cfg(test)]
use utoipa::{IntoParams, ToSchema};

use wastebin_core::crypto::Password;
use wastebin_core::db::{Head, read, write};
use wastebin_core::expiration::Expiration;
use wastebin_core::id::Id;

use crate::AppState;
use crate::cache::valid_extension;
use crate::client::ClientKey;
use crate::handlers::extract::sign_owner_token;
use crate::handlers::uid_cookie;
use crate::service;
use crate::v1::auth::{self, Identity};
#[cfg(test)]
use crate::v1::contract::responses::{
    BadRequest, Forbidden, NotFound, PayloadTooLarge, ReadPrecondition, TooManyRequests,
    Unauthorized, WrongPassword,
};
use crate::v1::error::{ApiError, ErrorType, V1Json};
#[cfg(test)]
use crate::v1::model::PasteId;
use crate::v1::model::{
    ContentDocument, ContentReadRequest, Paste, PasteCreate, PasteCreateRequest, PasteCreated,
    PasteCreatedDocument, PasteDocument, PasteListDocument, VERSION_DOCUMENT, VersionDocument,
};

const MAX_TITLE_CHARS: usize = 256;
const DEFAULT_LIST_LIMIT: usize = 20;
const MAX_LIST_LIMIT: usize = 100;

/// The query of `listPastes` as sent; [`list`] checks each value, so every malformed one is the
/// same 400 in the envelope.
#[derive(Deserialize)]
pub(crate) struct ListParams {
    owner: Option<String>,
    limit: Option<String>,
    marker: Option<String>,
}

/// Show the API version
#[cfg_attr(test, utoipa::path(
    get,
    path = "/v1",
    tag = "versions",
    operation_id = "getVersion",
    extensions(("x-phase" = json!("P1"))),
    security(()),
    responses(
        (status = 200, description = "The current API version.", body = VersionDocument,
            example = json!({"version": {"id": "v1", "status": "CURRENT"}})),
    ),
))]
pub(crate) async fn get_version() -> Json<VersionDocument> {
    Json(VERSION_DOCUMENT)
}

/// Create a paste
///
/// Credentials are optional. With a valid Bearer token or cookie the new
/// paste joins that identity, and the response carries no `owner_token` and
/// sets no cookie. An invalid Bearer token is 401; the server never falls back
/// to a new identity.
///
/// Without credentials the server mints a new identity. How it hands the
/// identity over depends on the caller:
///
/// - A browser create, recognised by an `Origin` equal to the configured
///   public URL, gets `Set-Cookie: uid=...; HttpOnly; Secure; SameSite=Lax;
///   Path=/` and no `owner_token` in the body, so page scripts never see the
///   token.
/// - Any other create (no `Origin`, as from curl and gzpaste) gets
///   `owner_token` in the body, once, and no cookie.
///
/// A create that sets `password` derives a key, so it counts against the
/// client's derivation rate and takes a slot of the shared key-derivation
/// limit; past either it is 429 with `Retry-After`, and nothing is created. A
/// create without a password never counts. The general create rate is the
/// proxy's.
#[cfg_attr(test, utoipa::path(
    post,
    path = "/v1/pastes",
    tag = "pastes",
    operation_id = "createPaste",
    extensions(("x-phase" = json!("P1"))),
    security((), ("ownerToken" = []), ("ownerCookie" = [])),
    request_body(content = PasteCreateRequest, example = json!({"paste": {
        "content": "# Plan\n",
        "extension": "md",
        "title": "Plan",
        "expires_in": 604_800,
        "burn_after_reading": false,
        "password": null
    }})),
    responses(
        (status = 201, body = PasteCreatedDocument,
            description = "Created. `owner_token` is present only when a non-browser create minted a new identity."),
        (status = 400, response = BadRequest),
        (status = 401, response = Unauthorized),
        (status = 403, response = Forbidden),
        (status = 413, response = PayloadTooLarge),
        (status = 429, response = TooManyRequests),
    ),
))]
pub(crate) async fn create(
    State(state): State<AppState>,
    client: ClientKey,
    headers: HeaderMap,
    body: Result<V1Json<PasteCreateRequest>, ApiError>,
) -> Result<Response, ApiError> {
    // Credentials and `Origin` are answered before the body.
    let caller = auth::caller(&headers, &state.key, &state.page.base_url, true)?;
    let V1Json(PasteCreateRequest { paste }) = body?;
    let entry = validate(paste, &state.page.expirations)?;

    let owner = match caller.identity {
        Identity::Token(uid) => Some(uid),
        Identity::Cookie(uids) => uids.first().copied(),
        Identity::Anonymous => None,
    };
    let (head, uid) = service::create(&state, client, entry, owner).await?;
    let (owner_token, cookie) = match owner {
        Some(_) => (None, None),
        None if caller.browser => (None, Some(uid_cookie(&[uid]))),
        None => (Some(sign_owner_token(&state.key, uid)), None),
    };
    let location = HeaderValue::try_from(format!("/v1/pastes/{}", head.id))
        .map_err(|_| ApiError::internal())?;
    let body = PasteCreatedDocument {
        paste: PasteCreated::new(&head, &state.page.base_url, owner_token),
    };
    let response = (
        StatusCode::CREATED,
        [(header::LOCATION, location), (header::ETAG, etag(&head))],
        Json(body),
    );

    Ok(match cookie {
        Some(cookie) => {
            let jar = SignedCookieJar::from_headers(&headers, state.key.clone()).add(cookie);
            (jar, response).into_response()
        }
        None => response.into_response(),
    })
}

/// Check a create body against the contract and turn it into a database entry.
fn validate(paste: PasteCreate, expirations: &[Expiration]) -> Result<write::Entry, ApiError> {
    if let Some(extension) = &paste.extension
        && (extension.0.is_empty() || !valid_extension(&extension.0))
    {
        return Err(ApiError::bad_request(
            "extension must be 1 to 32 characters of A-Z, a-z, 0-9, '.', '_' or '-'",
        ));
    }

    if paste
        .title
        .0
        .as_ref()
        .is_some_and(|title| title.chars().count() > MAX_TITLE_CHARS)
    {
        return Err(ApiError::bad_request("title exceeds 256 characters"));
    }

    if paste.password.as_ref().is_some_and(String::is_empty) {
        return Err(ApiError::bad_request("password must be null or non-empty"));
    }

    Ok(write::Entry {
        text: paste.content,
        extension: paste.extension.map(|extension| extension.0),
        expires: expiry(paste.expires_in, expirations)?,
        burn_after_reading: Some(paste.burn_after_reading),
        uid: None,
        password: paste.password,
        title: paste.title.0,
    })
}

/// Expiry for `expires_in`: one of the configured values, or the default when omitted. The
/// default is the value marked `=d`, else the first configured one; 0 means never.
fn expiry(
    expires_in: Option<u64>,
    expirations: &[Expiration],
) -> Result<Option<NonZeroU32>, ApiError> {
    let chosen = match expires_in {
        Some(secs) => expirations
            .iter()
            .find(|expiration| expiration.duration.as_secs() == secs)
            .ok_or_else(|| {
                ApiError::bad_request("expires_in is not one of the configured expirations")
            })?,
        None => match expirations
            .iter()
            .find(|expiration| expiration.default)
            .or_else(|| expirations.first())
        {
            Some(expiration) => expiration,
            None => return Ok(None),
        },
    };

    let secs = u32::try_from(chosen.duration.as_secs())
        .map_err(|_| ApiError::bad_request("expires_in is too large"))?;
    Ok(NonZeroU32::new(secs))
}

/// Show paste metadata
///
/// Returns whitelisted fields only: never the uid, and no title for an
/// encrypted paste. Reading metadata never consumes a burn-after-reading paste.
#[cfg_attr(test, utoipa::path(
    get,
    path = "/v1/pastes/{id}",
    tag = "pastes",
    operation_id = "showPaste",
    extensions(("x-phase" = json!("P1"))),
    security(()),
    responses(
        (status = 200, description = "The paste.", body = PasteDocument),
        (status = 404, response = NotFound),
    ),
))]
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let head = state.db.head(paste_id(&id)?).await?;
    let body = PasteDocument {
        paste: Paste::new(&head, &state.page.base_url),
    };

    Ok(([(header::ETAG, etag(&head))], Json(body)).into_response())
}

/// Delete a paste
///
/// Deletes the paste and its whole history.
#[cfg_attr(test, utoipa::path(
    delete,
    path = "/v1/pastes/{id}",
    tag = "pastes",
    operation_id = "deletePaste",
    extensions(("x-phase" = json!("P1"))),
    security(("ownerToken" = []), ("ownerCookie" = [])),
    responses(
        (status = 204, description = "Deleted."),
        (status = 401, response = Unauthorized),
        (status = 403, response = Forbidden),
        (status = 404, response = NotFound),
    ),
))]
pub(crate) async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let caller = auth::caller(&headers, &state.key, &state.page.base_url, true)?;
    let uids = caller.uids();
    if uids.is_empty() {
        return Err(ApiError::unauthorized(
            "an owner token or the uid cookie is required",
        ));
    }

    let head = state.db.head(paste_id(&id)?).await?;
    if !head.uid.is_some_and(|uid| uids.contains(&uid)) {
        return Err(ApiError::forbidden("the paste belongs to another identity"));
    }

    match state.db.delete_for(head.id, uids).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        // Deleted or expired since the ownership check.
        Err(wastebin_core::db::Error::Delete) => Err(ApiError::not_found()),
        Err(err) => Err(err.into()),
    }
}

/// Read the content of a plain paste
///
/// Returns the raw text of a paste that is neither encrypted nor
/// burn-after-reading. For an encrypted paste (burn or not) it answers 409
/// `password_required`; for a burn-after-reading paste, 409
/// `confirmation_required`. It consumes nothing.
#[cfg_attr(test, utoipa::path(
    get,
    path = "/v1/pastes/{id}/content",
    tag = "content",
    operation_id = "getContent",
    extensions(("x-phase" = json!("P1"))),
    security(()),
    responses(
        (status = 200, description = "The current content, UTF-8.", body = String,
            content_type = "text/plain; charset=utf-8"),
        (status = 404, response = NotFound),
        (status = 409, response = ReadPrecondition),
    ),
))]
pub(crate) async fn get_content(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let id = paste_id(&id)?;
    // The flags come first, so a 409 never copies the stored bytes.
    let head = state.db.head(id).await?;

    if head.encrypted {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            ErrorType::PasswordRequired,
            "the paste is encrypted; read it with POST /v1/pastes/{id}/content and its password",
        ));
    }

    if head.burn_after_reading {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            ErrorType::ConfirmationRequired,
            "confirm to read; the paste is deleted after reading",
        ));
    }

    // Both flags are fixed at creation, so they still hold for this second read; its head
    // carries the version the content belongs to.
    let (head, data) = state.db.head_with_data(id).await?;
    let text = read::decompress(data).await?;
    let content_type = HeaderValue::from_static("text/plain; charset=utf-8");

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::ETAG, etag(&head)),
        ],
        text,
    )
        .into_response())
}

// The documented values of `owner`; `ListParams` reads the query.
#[cfg(test)]
#[allow(dead_code, reason = "a schema of the generated contract")]
#[derive(Serialize, ToSchema)]
enum OwnerFilter {
    #[serde(rename = "me")]
    Me,
}

// The documented query of `listPastes`; `ListParams` reads it.
#[cfg(test)]
#[allow(dead_code, reason = "a schema of the generated contract")]
#[derive(Serialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ListQuery {
    /// Only `me` is defined.
    #[param(inline)]
    owner: OwnerFilter,
    #[param(minimum = 1, maximum = 100, default = 20)]
    limit: Option<u32>,
    /// Id of the last paste on the previous page. A marker that is malformed,
    /// deleted, expired or owned by another identity gives the same 400.
    marker: Option<PasteId>,
}

/// List the caller's pastes
///
/// Filters by identity and expiry first, then pages. Order is newest first by
/// `(created_at, id)`; pastes from before the `created_at` column sort last.
/// `pastes_links` holds only a `next` link, which keeps `owner=me` and `limit`.
/// With the cookie, `me` is every identity the cookie holds.
#[cfg_attr(test, utoipa::path(
    get,
    path = "/v1/pastes",
    tag = "pastes",
    operation_id = "listPastes",
    extensions(("x-phase" = json!("P1"))),
    security(("ownerToken" = []), ("ownerCookie" = [])),
    params(ListQuery),
    responses(
        (status = 200, description = "One page of pastes.", body = PasteListDocument),
        (status = 400, response = BadRequest),
        (status = 401, response = Unauthorized),
    ),
))]
pub(crate) async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: Result<Query<ListParams>, QueryRejection>,
) -> Result<Response, ApiError> {
    let caller = auth::caller(&headers, &state.key, &state.page.base_url, false)?;
    let uids = caller.uids();
    if uids.is_empty() {
        return Err(ApiError::unauthorized(
            "an owner token or the uid cookie is required",
        ));
    }

    let Query(params) = query.map_err(|rejection| ApiError::bad_request(rejection.body_text()))?;
    if params.owner.as_deref() != Some("me") {
        return Err(ApiError::bad_request("owner must be me"));
    }
    let limit = match params.limit.as_deref() {
        None => DEFAULT_LIST_LIMIT,
        Some(limit) => limit
            .parse::<usize>()
            .ok()
            .filter(|limit| (1..=MAX_LIST_LIMIT).contains(limit))
            .ok_or_else(|| ApiError::bad_request("limit must be an integer from 1 to 100"))?,
    };
    let marker = params
        .marker
        .as_deref()
        .map(|marker| marker.parse::<Id>().map_err(|_| invalid_marker()))
        .transpose()?;

    let page = match state.db.list_for(uids, marker, limit).await {
        Ok(page) => page,
        Err(wastebin_core::db::Error::InvalidMarker) => return Err(invalid_marker()),
        Err(err) => return Err(err.into()),
    };
    let next = page
        .more
        .then(|| page.heads.last())
        .flatten()
        .map(|last| next_query(limit, last.id));

    Ok(Json(PasteListDocument::new(
        &page.heads,
        &state.page.base_url,
        next,
    ))
    .into_response())
}

/// The query of the page after `marker`. Ids may hold `+`, which a query decodes as a space unless
/// it is encoded.
fn next_query(limit: usize, marker: Id) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .append_pair("owner", "me")
        .append_pair("limit", &limit.to_string())
        .append_pair("marker", &marker.to_string())
        .finish()
}

/// One answer for every marker that is not an unexpired paste of the caller, so a marker reveals
/// nothing about an id (decision D6).
fn invalid_marker() -> ApiError {
    ApiError::bad_request("marker is not a paste of this list")
}

/// Read content with a password or a burn confirmation
///
/// No credentials are needed, but a request that carries the `uid` cookie
/// without a matching `Origin` is 403 before the paste is looked up, because
/// a burn read is a write.
///
/// The only way to consume a burn-after-reading paste: it is consumed once,
/// atomically, when the request has `confirm_burn: true` (and, if encrypted,
/// the right `password`), and is 404 afterwards. Of two concurrent confirmed
/// requests, one gets the content and the other 404. A client that gets 404
/// on a retry must treat the paste as possibly read and must not retry on its
/// own.
///
/// | Paste | Body | Answer |
/// | --- | --- | --- |
/// | encrypted | no `password` | 409 `password_required` |
/// | encrypted | wrong `password` | 401 `wrong_password` |
/// | encrypted, not burn | right `password` | 200 |
/// | burn, not encrypted | no `confirm_burn` | 409 `confirmation_required` |
/// | burn, not encrypted | `confirm_burn: true` | 200, consumed |
/// | encrypted and burn | right `password`, no `confirm_burn` | 409 `confirmation_required`; nothing returned or consumed |
/// | encrypted and burn | right `password` and `confirm_burn: true` | 200, consumed |
///
/// Every request that carries `password` for an encrypted paste derives a
/// key, the confirmation step included: it counts against the client's
/// derivation rate and takes a slot of the shared derivation limit, and past
/// either it is 429 with `Retry-After`. A plain paste derives nothing.
#[cfg_attr(test, utoipa::path(
    post,
    path = "/v1/pastes/{id}/content",
    tag = "content",
    operation_id = "readContent",
    extensions(("x-phase" = json!("P1"))),
    security(()),
    request_body(content = ContentReadRequest, examples(
        ("password" = (value = json!({"password": "hunter2"}))),
        ("burn" = (value = json!({"confirm_burn": true}))),
        ("both" = (value = json!({"password": "hunter2", "confirm_burn": true}))),
    )),
    responses(
        (status = 200, description = "The current content.", body = ContentDocument),
        (status = 400, response = BadRequest),
        (status = 401, response = WrongPassword),
        (status = 403, response = Forbidden),
        (status = 404, response = NotFound),
        (status = 409, response = ReadPrecondition),
        (status = 429, response = TooManyRequests),
    ),
))]
pub(crate) async fn read_content(
    State(state): State<AppState>,
    Path(id): Path<String>,
    client: ClientKey,
    headers: HeaderMap,
    body: Result<V1Json<ContentReadRequest>, ApiError>,
) -> Result<Response, ApiError> {
    // `Origin` and the body are answered before the paste is looked up or a key derived.
    auth::check_origin(&headers, &state.page.base_url)?;
    let V1Json(request) = body?;
    if request.password.as_ref().is_some_and(String::is_empty) {
        return Err(ApiError::bad_request("password must be non-empty"));
    }
    let password = request
        .password
        .map(|password| Password::from(password.into_bytes()));

    let content = service::read(
        &state,
        client,
        paste_id(&id)?,
        password,
        request.confirm_burn.is_some(),
    )
    .await?;

    Ok((
        [(header::ETAG, etag(&content.head))],
        Json(ContentDocument::new(&content.head, content.text)),
    )
        .into_response())
}

/// A path id that does not parse names no paste: 404, like a missing one.
fn paste_id(id: &str) -> Result<Id, ApiError> {
    id.parse().map_err(|_| ApiError::not_found())
}

/// Strong entity tag of the head version, `"<version_id>"`.
fn etag(head: &Head) -> HeaderValue {
    HeaderValue::try_from(format!("\"{}\"", head.version_id))
        .unwrap_or_else(|_| HeaderValue::from_static("\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_query_survives_a_plus_in_the_marker() -> Result<(), Box<dyn std::error::Error>> {
        let marker: Id = "ab+c-1".parse()?;
        let id = marker.to_string();
        assert!(id.contains('+'), "{id}");
        let query = next_query(2, marker);
        let decoded = url::form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect::<Vec<_>>();
        assert_eq!(
            decoded,
            [
                (String::from("owner"), String::from("me")),
                (String::from("limit"), String::from("2")),
                (String::from("marker"), id),
            ]
        );
        Ok(())
    }
}
