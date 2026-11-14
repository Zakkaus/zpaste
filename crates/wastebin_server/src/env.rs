use std::env::VarError;
use std::fmt::Display;
use std::net::{Ipv4Addr, SocketAddr};
use std::num::{NonZeroU32, NonZeroUsize, ParseIntError};
use std::path::PathBuf;
use std::time::Duration;

use axum_extra::extract::cookie::Key;

use wastebin_core::crypto::KdfLimit;
use wastebin_core::env::vars::{
    self, ADDRESS_PORT, BASE_URL, CACHE_SIZE, DATABASE_PATH, HTTP_TIMEOUT, KDF_CONCURRENCY,
    KDF_RATE, MAX_BODY_SIZE, PASTE_EXPIRATIONS, SIGNING_KEY, TRUSTED_PROXIES,
};
use wastebin_core::{db, expiration};
use wastebin_highlight::{Theme, theme::ParseThemeNameError};

use crate::client::{RateLimit, TrustedProxies};

pub const DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("failed to construct cache")]
    CacheConstruction(#[from] cached::BuildError),
    #[error("failed to parse {CACHE_SIZE}, expected number of elements: {0}")]
    CacheSize(ParseIntError),
    #[error("failed to parse {DATABASE_PATH}, contains non-Unicode data")]
    DatabasePath,
    #[error("failed to parse {MAX_BODY_SIZE}, expected number of bytes: {0}")]
    MaxBodySize(ParseIntError),
    #[error("failed to parse {ADDRESS_PORT}, expected `host:port`")]
    AddressPort,
    #[error("failed to parse {BASE_URL}: {0}")]
    BaseUrl(String),
    #[error("failed to generate key from {SIGNING_KEY}: {0}")]
    SigningKey(String),
    #[error("failed to parse {KDF_CONCURRENCY}, expected a positive number: {0}")]
    KdfConcurrency(ParseIntError),
    #[error("failed to parse {KDF_RATE}, expected a positive number: {0}")]
    KdfRate(ParseIntError),
    #[error("failed to parse {TRUSTED_PROXIES}: {0}")]
    TrustedProxies(String),
    #[error("failed to parse {HTTP_TIMEOUT}: {0}")]
    HttpTimeout(ParseIntError),
    #[error("failed to parse {PASTE_EXPIRATIONS}: {0}")]
    ParsePasteExpiration(#[from] expiration::Error),
    #[error("failed to parse theme name")]
    ParseTheme(#[from] ParseThemeNameError),
    #[error("binding to both TCP and Unix socket is not possible")]
    BothListeners,
}

pub(crate) enum SocketType {
    Tcp(SocketAddr),
    Unix(PathBuf),
}

impl Display for SocketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SocketType::Tcp(addr) => {
                write!(f, "{addr}")
            }
            SocketType::Unix(path) => {
                write!(f, "{}", path.display())
            }
        }
    }
}

pub fn title() -> String {
    std::env::var(vars::TITLE).unwrap_or_else(|_| "wastebin".to_string())
}

pub fn theme() -> Result<Theme, Error> {
    Ok(std::env::var(vars::THEME).map_or_else(|_| Ok(Theme::Ayu), |var| var.parse())?)
}

pub fn cache_size() -> Result<NonZeroUsize, Error> {
    std::env::var(vars::CACHE_SIZE)
        .map_or_else(
            |_| Ok(NonZeroUsize::new(128).expect("128 is non-zero")),
            |s| s.parse::<NonZeroUsize>(),
        )
        .map_err(Error::CacheSize)
}

pub fn database_method() -> Result<db::Open, Error> {
    match std::env::var(vars::DATABASE_PATH) {
        Ok(path) => Ok(db::Open::Path(PathBuf::from(path))),
        Err(VarError::NotUnicode(_)) => Err(Error::DatabasePath),
        Err(VarError::NotPresent) => Ok(db::Open::Memory),
    }
}

pub fn signing_key() -> Result<Key, Error> {
    std::env::var(vars::SIGNING_KEY).map_or_else(
        |_| Ok(Key::generate()),
        |s| Key::try_from(s.as_bytes()).map_err(|err| Error::SigningKey(err.to_string())),
    )
}

pub fn socket_type() -> Result<SocketType, Error> {
    match (
        std::env::var(vars::ADDRESS_PORT),
        std::env::var(vars::SOCKET_PATH),
    ) {
        (Ok(_), Ok(_)) => Err(Error::BothListeners),
        (Ok(var), Err(_)) => {
            let addr: SocketAddr = var.parse().map_err(|_| Error::AddressPort)?;
            Ok(SocketType::Tcp(addr))
        }
        (Err(_), Ok(var)) => Ok(SocketType::Unix(var.into())),
        (Err(_), Err(_)) => {
            let addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 8088);
            Ok(SocketType::Tcp(addr))
        }
    }
}

pub fn max_body_size() -> Result<usize, Error> {
    std::env::var(vars::MAX_BODY_SIZE)
        .map_or_else(|_| Ok(1024 * 1024), |s| s.parse::<usize>())
        .map_err(Error::MaxBodySize)
}

/// Read base URL either from the environment variable or fallback to the hostname.
pub fn base_url() -> Result<url::Url, Error> {
    if let Some(base_url) = std::env::var(vars::BASE_URL).map_or_else(
        |err| {
            if matches!(err, VarError::NotUnicode(_)) {
                Err(Error::BaseUrl(format!("{BASE_URL} is not unicode")))
            } else {
                Ok(None)
            }
        },
        |var| {
            Ok(Some(
                url::Url::parse(&var).map_err(|err| Error::BaseUrl(err.to_string()))?,
            ))
        },
    )? {
        return Ok(base_url);
    }

    let hostname =
        hostname::get().map_err(|err| Error::BaseUrl(format!("failed to get hostname: {err}")))?;

    url::Url::parse(&format!("https://{}", hostname.to_string_lossy()))
        .map_err(|err| Error::BaseUrl(err.to_string()))
}

pub fn http_timeout() -> Result<Duration, Error> {
    std::env::var(vars::HTTP_TIMEOUT)
        .map_or_else(
            |_| Ok(DEFAULT_HTTP_TIMEOUT),
            |s| s.parse::<u64>().map(|v| Duration::new(v, 0)),
        )
        .map_err(Error::HttpTimeout)
}

/// Limit on concurrent password key derivations; [`KdfLimit::DEFAULT_CONCURRENCY`] unless set.
pub fn kdf_limit() -> Result<KdfLimit, Error> {
    let concurrency = std::env::var(vars::KDF_CONCURRENCY)
        .map_or_else(
            |_| Ok(KdfLimit::DEFAULT_CONCURRENCY),
            |s| s.parse::<NonZeroUsize>(),
        )
        .map_err(Error::KdfConcurrency)?;

    Ok(KdfLimit::new(concurrency, KdfLimit::DEFAULT_WAIT))
}

/// Per-client rate of key derivations, [`RateLimit::DEFAULT_PER_MINUTE`] when unset.
pub fn kdf_rate() -> Result<RateLimit, Error> {
    let rate = std::env::var(vars::KDF_RATE)
        .map_or_else(
            |_| Ok(RateLimit::DEFAULT_PER_MINUTE),
            |s| s.parse::<NonZeroU32>(),
        )
        .map_err(Error::KdfRate)?;

    Ok(RateLimit::per_minute(rate))
}

/// Trusted proxies, none when unset.
pub fn trusted_proxies() -> Result<TrustedProxies, Error> {
    std::env::var(vars::TRUSTED_PROXIES)
        .map_or_else(|_| Ok(TrustedProxies::default()), |s| s.parse())
        .map_err(Error::TrustedProxies)
}

/// Parse [`expiration::ExpirationSet`] from environment or return default.
pub fn expiration_set() -> Result<expiration::ExpirationSet, Error> {
    let set = std::env::var(vars::PASTE_EXPIRATIONS).map_or_else(
        |_| "0=d,10m,1h,1d,1w,1M,1y".parse::<expiration::ExpirationSet>(),
        |value| value.parse::<expiration::ExpirationSet>(),
    )?;

    Ok(set)
}
