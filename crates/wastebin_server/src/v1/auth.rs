//! Credential selection and the `Origin` rule, the same on every `/v1` operation (contract
//! `info.description`, "Credential selection").

use axum::http::{HeaderMap, header};
use axum_extra::extract::cookie::Key;
use axum_extra::extract::{CookieJar, SignedCookieJar};
use url::Url;

use crate::handlers::extract::{parse_uids, verify_owner_token};
use crate::v1::error::ApiError;

/// The identity a request acts as.
pub(crate) enum Identity {
    /// A verified `Authorization: Bearer` owner token.
    Token(i64),
    /// The identities of a `uid` cookie whose signature verified.
    Cookie(Vec<i64>),
    /// No credentials.
    Anonymous,
}

pub(crate) struct Caller {
    pub identity: Identity,
    /// `Origin` equals the public URL: the request comes from the web app.
    pub browser: bool,
}

impl Caller {
    /// Every uid the caller holds.
    pub(crate) fn uids(&self) -> &[i64] {
        match &self.identity {
            Identity::Token(uid) => std::slice::from_ref(uid),
            Identity::Cookie(uids) => uids,
            Identity::Anonymous => &[],
        }
    }
}

/// Select the caller's credentials.
///
/// `writes` marks a request that creates, edits, deletes or consumes; such a request that carries
/// the `uid` cookie without `Authorization` needs `Origin` equal to `public_url`, checked before
/// the cookie's signature or anything else.
pub(crate) fn caller(
    headers: &HeaderMap,
    key: &Key,
    public_url: &Url,
    writes: bool,
) -> Result<Caller, ApiError> {
    let browser = is_browser(headers, public_url);

    // With `Authorization`, the header alone decides and the cookie is ignored.
    if let Some(authorization) = headers.get(header::AUTHORIZATION) {
        let uid = authorization
            .to_str()
            .ok()
            .and_then(|value| value.split_once(' '))
            .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
            .and_then(|(_, token)| verify_owner_token(key, token.trim()))
            .ok_or_else(|| ApiError::unauthorized("invalid owner token"))?;

        return Ok(Caller {
            identity: Identity::Token(uid),
            browser,
        });
    }

    if CookieJar::from_headers(headers).get("uid").is_none() {
        return Ok(Caller {
            identity: Identity::Anonymous,
            browser,
        });
    }

    if writes && !browser {
        return Err(origin_mismatch());
    }

    // A cookie whose signature fails counts as absent, as in upstream's signed cookie jar.
    let identity = SignedCookieJar::from_headers(headers, key.clone())
        .get("uid")
        .map(|cookie| parse_uids(cookie.value_trimmed()))
        .filter(|uids| !uids.is_empty())
        .map_or(Identity::Anonymous, Identity::Cookie);

    Ok(Caller { identity, browser })
}

/// The `Origin` rule alone, for an operation that writes but takes no credentials
/// (`readContent`, which can consume a paste): a request with the `uid` cookie and without
/// `Authorization` needs `Origin` equal to `public_url`. `Authorization` is not verified, because
/// the operation grants it nothing.
pub(crate) fn check_origin(headers: &HeaderMap, public_url: &Url) -> Result<(), ApiError> {
    if headers.contains_key(header::AUTHORIZATION)
        || CookieJar::from_headers(headers).get("uid").is_none()
        || is_browser(headers, public_url)
    {
        Ok(())
    } else {
        Err(origin_mismatch())
    }
}

/// `Origin` equals the public URL: the request comes from the web app.
fn is_browser(headers: &HeaderMap, public_url: &Url) -> bool {
    headers
        .get(header::ORIGIN)
        .and_then(|origin| origin.to_str().ok())
        .and_then(|origin| Url::parse(origin).ok())
        .is_some_and(|origin| origin.origin() == public_url.origin())
}

fn origin_mismatch() -> ApiError {
    ApiError::forbidden("a request with the uid cookie must send Origin equal to this site")
}
