//! The single error envelope of `/v1`, the `X-Request-Id` and `Cache-Control` headers every `/v1`
//! response carries, and the conversion of framework rejections into the envelope.

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde::de::DeserializeOwned;
use utoipa::ToSchema;

use wastebin_core::db;

pub(crate) const REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Challenge sent with a 401 for a missing or failed owner token.
pub(crate) const BEARER_CHALLENGE: HeaderValue =
    HeaderValue::from_static("Bearer realm=\"zpaste\"");

/// Machine-readable kind. The plan names `version_conflict`,
/// `password_required` and `confirmation_required`; the rest are this
/// contract's. `docs/api/README.md` maps each operation and status to
/// its types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[allow(
    dead_code,
    reason = "the contract's closed list; later phases answer the other types"
)]
pub(crate) enum ErrorType {
    BadRequest,
    Unauthorized,
    WrongPassword,
    Forbidden,
    NotFound,
    PasswordRequired,
    ConfirmationRequired,
    BurnNotEditable,
    BurnNoHistory,
    VersionConflict,
    SlugTaken,
    EntryNameTaken,
    PasteInWorkspace,
    WorkspaceFull,
    PayloadTooLarge,
    RateLimited,
    MethodNotAllowed,
    UnsupportedMediaType,
    InternalError,
    ServiceUnavailable,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = Error)]
pub(crate) struct ErrorBody {
    /// The HTTP status code.
    code: u16,
    #[serde(rename = "type")]
    #[schema(inline)]
    kind: ErrorType,
    /// The HTTP reason phrase.
    title: String,
    /// Human-readable detail. Carries no internal or upstream detail.
    message: String,
    /// Same value as the `X-Request-Id` response header.
    request_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ErrorDocument {
    error: ErrorBody,
}

/// An error answered by a `/v1` handler. The envelope middleware renders it, because only the
/// middleware knows the request id.
#[derive(Clone, Debug)]
pub(crate) struct ApiError {
    status: StatusCode,
    kind: ErrorType,
    message: String,
    challenge: Option<HeaderValue>,
}

impl ApiError {
    pub(crate) fn new(status: StatusCode, kind: ErrorType, message: impl Into<String>) -> Self {
        Self {
            status,
            kind,
            message: message.into(),
            challenge: None,
        }
    }

    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, ErrorType::BadRequest, message)
    }

    pub(crate) fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            challenge: Some(BEARER_CHALLENGE),
            ..Self::new(StatusCode::UNAUTHORIZED, ErrorType::Unauthorized, message)
        }
    }

    pub(crate) fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, ErrorType::Forbidden, message)
    }

    pub(crate) fn not_found() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            ErrorType::NotFound,
            "paste does not exist or has expired",
        )
    }

    pub(crate) fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorType::InternalError,
            "internal error",
        )
    }

    /// The error for a status that no handler chose, such as a routing 404 or 405.
    fn from_status(status: StatusCode) -> Option<Self> {
        let (kind, message) = match status {
            StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => {
                (ErrorType::BadRequest, "malformed request")
            }
            StatusCode::NOT_FOUND => (ErrorType::NotFound, "no such resource"),
            StatusCode::METHOD_NOT_ALLOWED => (
                ErrorType::MethodNotAllowed,
                "method not allowed on this resource",
            ),
            StatusCode::PAYLOAD_TOO_LARGE => (
                ErrorType::PayloadTooLarge,
                "request body exceeds the size limit",
            ),
            StatusCode::UNSUPPORTED_MEDIA_TYPE => {
                (ErrorType::UnsupportedMediaType, "unsupported content type")
            }
            StatusCode::SERVICE_UNAVAILABLE => {
                (ErrorType::ServiceUnavailable, "service unavailable")
            }
            status if status.is_server_error() => (ErrorType::InternalError, "internal error"),
            _ => return None,
        };

        // A body that fails validation is 400, not axum's 422 (contract decision D8).
        let status = if status == StatusCode::UNPROCESSABLE_ENTITY {
            StatusCode::BAD_REQUEST
        } else {
            status
        };

        Some(Self::new(status, kind, message))
    }
}

impl From<db::Error> for ApiError {
    fn from(err: db::Error) -> Self {
        match err {
            db::Error::NotFound => Self::not_found(),
            err => {
                tracing::error!(%err, "database error on /v1");
                Self::internal()
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = self.status.into_response();
        if let Some(challenge) = &self.challenge {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, challenge.clone());
        }
        response.extensions_mut().insert(self);
        response
    }
}

/// `Json` that answers a body it cannot read with the `/v1` envelope: 400 for malformed JSON or a
/// field that fails to deserialize, 415 without a JSON content type, 413 above the body limit.
pub(crate) struct V1Json<T>(pub T);

impl<S, T> FromRequest<S> for V1Json<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(JsonRejection::MissingJsonContentType(_)) => Err(ApiError::new(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                ErrorType::UnsupportedMediaType,
                "expected Content-Type: application/json",
            )),
            Err(JsonRejection::JsonDataError(err)) => Err(ApiError::bad_request(err.body_text())),
            Err(JsonRejection::JsonSyntaxError(err)) => Err(ApiError::bad_request(err.body_text())),
            Err(rejection) if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE => {
                Err(ApiError::from_status(StatusCode::PAYLOAD_TOO_LARGE)
                    .unwrap_or_else(ApiError::internal))
            }
            Err(rejection) => Err(ApiError::bad_request(rejection.body_text())),
        }
    }
}

/// Whether `path` belongs to the `/v1` API.
pub(crate) fn is_v1(path: &str) -> bool {
    path == "/v1" || path.starts_with("/v1/")
}

/// Request id kept from a well-formed `X-Request-Id`, or a new one.
fn request_id(headers: &HeaderMap) -> HeaderValue {
    headers
        .get(REQUEST_ID)
        .filter(|value| {
            let bytes = value.as_bytes();
            (1..=64).contains(&bytes.len())
                && bytes
                    .iter()
                    .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
        })
        .cloned()
        .unwrap_or_else(|| {
            HeaderValue::try_from(format!("{:032x}", rand::random::<u128>()))
                .unwrap_or_else(|_| HeaderValue::from_static("0"))
        })
}

/// Middleware for every `/v1` response: echo the request id, forbid caching, and render every
/// error, a handler's or the framework's, as the one envelope.
pub(crate) async fn envelope(req: Request, next: Next) -> Response {
    if !is_v1(req.uri().path()) {
        return next.run(req).await;
    }

    let request_id = request_id(req.headers());
    let response = next.run(req).await;
    let status = response.status();

    let mut response = if status.is_client_error() || status.is_server_error() {
        let error = response
            .extensions()
            .get::<ApiError>()
            .cloned()
            .or_else(|| ApiError::from_status(status));

        match error {
            Some(error) => render(response, error, &request_id),
            None => response,
        }
    } else {
        response
    };

    let headers = response.headers_mut();
    headers.insert(REQUEST_ID, request_id);
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn render(response: Response, error: ApiError, request_id: &HeaderValue) -> Response {
    let (mut parts, _body) = response.into_parts();
    parts.headers.remove(header::CONTENT_TYPE);
    parts.headers.remove(header::CONTENT_LENGTH);
    if let Some(challenge) = error.challenge {
        parts.headers.insert(header::WWW_AUTHENTICATE, challenge);
    }

    let body = ErrorDocument {
        error: ErrorBody {
            code: error.status.as_u16(),
            kind: error.kind,
            title: error
                .status
                .canonical_reason()
                .unwrap_or_default()
                .to_owned(),
            message: error.message,
            request_id: request_id.to_str().unwrap_or_default().to_owned(),
        },
    };

    let mut response = (error.status, Json(body)).into_response();
    for (name, value) in &parts.headers {
        if !response.headers().contains_key(name) {
            response.headers_mut().insert(name, value.clone());
        }
    }
    response
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::{ApiError, ErrorType};

    #[test]
    fn unprocessable_entity_becomes_bad_request() {
        let error = ApiError::from_status(StatusCode::UNPROCESSABLE_ENTITY).expect("mapped");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.kind, ErrorType::BadRequest);
    }

    #[test]
    fn statuses_without_a_type_keep_their_body() {
        assert!(ApiError::from_status(StatusCode::REQUEST_TIMEOUT).is_none());
    }
}
