//! `/v1` handlers of this phase. Each `#[utoipa::path]` is the contract of its operation;
//! `docs/api/openapi.yaml` is generated from these annotations by the tests (see
//! `docs/api/README.md`).

use std::num::NonZeroU32;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::SignedCookieJar;

use wastebin_core::db::{Head, write};
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
    BadRequest, Forbidden, NotFound, PayloadTooLarge, TooManyRequests, Unauthorized,
};
use crate::v1::error::{ApiError, V1Json};
use crate::v1::model::{
    Paste, PasteCreate, PasteCreateRequest, PasteCreated, PasteCreatedDocument, PasteDocument,
    VERSION_DOCUMENT, VersionDocument,
};

const MAX_TITLE_CHARS: usize = 256;

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

/// A path id that does not parse names no paste: 404, like a missing one.
pub(super) fn paste_id(id: &str) -> Result<Id, ApiError> {
    id.parse().map_err(|_| ApiError::not_found())
}

/// Strong entity tag of the head version, `"<version_id>"`.
pub(super) fn etag(head: &Head) -> HeaderValue {
    HeaderValue::try_from(format!("\"{}\"", head.version_id))
        .unwrap_or_else(|_| HeaderValue::from_static("\"\""))
}
