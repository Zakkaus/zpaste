use std::num::TryFromIntError;

use axum::Json;
use axum::http::{HeaderValue, StatusCode};
use serde::Serialize;

use wastebin_core::{crypto, db, id};

use crate::client::retry_after_secs;
use crate::service::Failure;

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("axum http error: {0}")]
    Axum(#[from] axum::http::Error),
    #[error("integer conversion error: {0}")]
    IntConversion(#[from] TryFromIntError),
    #[error("join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("highlighting error: {0}")]
    SyntaxHighlighting(#[from] wastebin_highlight::Error),
    #[error("could not generate QR code: {0}")]
    QrCode(#[from] qrcodegen::DataTooLong),
    #[error("could not parse URL: {0}")]
    UrlParsing(#[from] url::ParseError),
    #[error("database error: {0}")]
    Database(#[from] db::Error),
    #[error("id error: {0}")]
    Id(#[from] id::Error),
    #[error("payload too large")]
    MalformedForm,
    #[error("{0}")]
    Service(#[from] Failure),
}

impl Error {
    /// `Retry-After` of a 429 from the per-client rate, which knows when a token is free again.
    /// The middleware gives every other 429 [`RETRY_AFTER`].
    pub(crate) fn retry_after(&self) -> Option<HeaderValue> {
        match self {
            Self::Service(Failure::RateLimited(wait)) => Some(retry_after_secs(*wait).into()),
            _ => None,
        }
    }
}

/// `Retry-After` of a 429 from the key-derivation limit, in seconds: about one derivation.
pub(crate) const RETRY_AFTER: HeaderValue = HeaderValue::from_static("1");

#[derive(Serialize)]
pub(crate) struct JsonError {
    pub message: String,
}

/// Response carrying a status code and the error message as JSON.
pub(crate) type JsonErrorResponse = (StatusCode, Json<JsonError>);

impl From<Error> for StatusCode {
    fn from(err: Error) -> Self {
        match err {
            Error::Database(db::Error::NotFound) => StatusCode::NOT_FOUND,
            Error::Database(db::Error::Crypto(crypto::Error::Busy)) => {
                StatusCode::TOO_MANY_REQUESTS
            }
            Error::Database(
                db::Error::Delete | db::Error::Crypto(crypto::Error::ChaCha20Poly1305Decrypt),
            ) => StatusCode::FORBIDDEN,
            Error::Database(db::Error::NoPassword) | Error::Id(_) | Error::UrlParsing(_) => {
                StatusCode::BAD_REQUEST
            }
            Error::MalformedForm => StatusCode::UNPROCESSABLE_ENTITY,
            // The legacy statuses of the same outcomes, as upstream answered them.
            Error::Service(failure) => match failure {
                Failure::NotFound => StatusCode::NOT_FOUND,
                Failure::PasswordRequired => StatusCode::BAD_REQUEST,
                Failure::WrongPassword => StatusCode::FORBIDDEN,
                Failure::ConfirmationRequired => StatusCode::CONFLICT,
                Failure::RateLimited(_) | Failure::Busy => StatusCode::TOO_MANY_REQUESTS,
                Failure::Database(err) => Error::Database(err).into(),
            },
            Error::Join(_)
            | Error::QrCode(_)
            | Error::Database(_)
            | Error::IntConversion(_)
            | Error::SyntaxHighlighting(_)
            | Error::Axum(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<Error> for JsonErrorResponse {
    fn from(err: Error) -> Self {
        let payload = Json::from(JsonError {
            message: err.to_string(),
        });

        (err.into(), payload)
    }
}

/// A legacy JSON error, with `Retry-After` when the per-client rate refused it.
pub(crate) fn json_error(err: Error) -> axum::response::Response {
    use axum::response::IntoResponse;

    let retry_after = err.retry_after();
    let mut response = JsonErrorResponse::from(err).into_response();
    if let Some(retry_after) = retry_after {
        response
            .headers_mut()
            .insert(http::header::RETRY_AFTER, retry_after);
    }
    response
}
