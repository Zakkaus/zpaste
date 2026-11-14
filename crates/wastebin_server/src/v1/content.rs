//! `/v1/pastes/{id}/content`: reading the current content without and with a password or a
//! burn confirmation.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

use wastebin_core::crypto::Password;

use crate::AppState;
use crate::client::ClientKey;
use crate::service;
use crate::v1::auth;
#[cfg(test)]
use crate::v1::contract::responses::{
    BadRequest, Forbidden, NotFound, ReadPrecondition, TooManyRequests, WrongPassword,
};
use crate::v1::error::{ApiError, ErrorType, V1Json};
use crate::v1::model::{ContentDocument, ContentReadRequest};
use crate::v1::pastes::{etag, paste_id};

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
    let (head, text) = service::peek(&state, paste_id(&id)?).await?;
    let Some(text) = text else {
        return Err(if head.encrypted {
            ApiError::new(
                StatusCode::CONFLICT,
                ErrorType::PasswordRequired,
                "the paste is encrypted; read it with POST /v1/pastes/{id}/content and its password",
            )
        } else {
            ApiError::new(
                StatusCode::CONFLICT,
                ErrorType::ConfirmationRequired,
                "confirm to read; the paste is deleted after reading",
            )
        });
    };
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
