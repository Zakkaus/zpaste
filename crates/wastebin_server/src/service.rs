//! Paste operations shared by `/v1` and the legacy routes (plan section 四, 舊路由與契約). Each
//! route only translates its request into these calls and their outcome into its own response, so
//! both answer the same scenario the same way.

use std::time::Duration;

use wastebin_core::crypto::{self, Password};
use wastebin_core::db::{self, Head, write};
use wastebin_core::id::Id;

use crate::AppState;
use crate::client::{ClientKey, RateLimit};

/// Why an operation did not succeed, before either route chooses its status.
#[derive(thiserror::Error, Debug)]
pub(crate) enum Failure {
    #[error("paste does not exist or has expired")]
    NotFound,
    #[error("the paste is encrypted and needs its password")]
    PasswordRequired,
    #[error("wrong password")]
    WrongPassword,
    #[error("the paste is deleted after reading; confirm to read it")]
    ConfirmationRequired,
    /// The client spent its derivation rate; a token is free again after the duration.
    #[error("too many password attempts from this client")]
    RateLimited(Duration),
    /// Every slot of the process-wide derivation limit stayed taken.
    #[error("too many password operations in progress")]
    Busy,
    #[error(transparent)]
    Database(db::Error),
}

impl From<db::Error> for Failure {
    fn from(err: db::Error) -> Self {
        match err {
            db::Error::NotFound => Self::NotFound,
            db::Error::NoPassword => Self::PasswordRequired,
            db::Error::Crypto(crypto::Error::ChaCha20Poly1305Decrypt) => Self::WrongPassword,
            db::Error::Crypto(crypto::Error::Busy) => Self::Busy,
            err => Self::Database(err),
        }
    }
}

/// Content read by [`read`].
pub(crate) struct Content {
    /// The paste as it was read; for a consumed paste, as it was just before.
    pub head: Head,
    pub text: String,
}

/// Count one key derivation against `client`'s rate. Call it only right before a derivation, after
/// every credential, `Origin` and body check.
pub(crate) fn admit(rate: &RateLimit, client: ClientKey) -> Result<(), Failure> {
    rate.take(client).map_err(Failure::RateLimited)
}

/// Create a paste owned by `owner`, or by a newly minted identity when `owner` is `None`, and
/// return it with the owner's uid. A password derives a key, so it counts against `client`'s rate
/// before an identity is minted.
pub(crate) async fn create(
    state: &AppState,
    client: ClientKey,
    mut entry: write::Entry,
    owner: Option<i64>,
) -> Result<(Head, i64), Failure> {
    if entry.password.is_some() {
        admit(&state.rate, client)?;
    }

    let uid = match owner {
        Some(uid) => uid,
        None => state.db.next_uid().await?,
    };
    entry.uid = Some(uid);

    Ok((state.db.create(entry).await?, uid))
}

/// The head of `id` and, for an ordinary paste, its current text. An encrypted or
/// burn-after-reading paste gives no text: its stored bytes are not even read, and nothing is
/// derived or consumed.
pub(crate) async fn peek(state: &AppState, id: Id) -> Result<(Head, Option<String>), Failure> {
    // The flags come first, so a sealed paste's bytes are never copied.
    let head = state.db.head(id).await?;
    if head.encrypted || head.burn_after_reading {
        return Ok((head, None));
    }

    // Both flags are fixed at creation, so they still hold for this second read; its head
    // carries the version the text belongs to.
    let (head, data) = state.db.head_with_data(id).await?;
    let text = db::read::decompress(data).await?;
    Ok((head, Some(text)))
}

/// Read the current content of `id`.
///
/// An encrypted paste needs `password`; checking it derives a key, which counts against
/// `client`'s rate and takes a slot of the shared limit, and consumes nothing. A
/// burn-after-reading paste needs `confirm_burn`, checked after the password, and is then
/// consumed atomically: of concurrent confirmed reads exactly one gets the content and the others
/// [`Failure::NotFound`]. A plain paste derives nothing and ignores `password`.
pub(crate) async fn read(
    state: &AppState,
    client: ClientKey,
    id: Id,
    password: Option<Password>,
    confirm_burn: bool,
) -> Result<Content, Failure> {
    let (head, stored) = state.db.head_with_stored(id).await?;

    if head.encrypted {
        let password = password.ok_or(Failure::PasswordRequired)?;
        admit(&state.rate, client)?;
        let text = state.db.open(stored, Some(password)).await?;

        if !head.burn_after_reading {
            return Ok(Content { head, text });
        }
        if !confirm_burn {
            return Err(Failure::ConfirmationRequired);
        }

        // A burn paste cannot be edited, so the consumed version is the one just verified.
        let (head, _) = state.db.consume(id).await?;
        return Ok(Content { head, text });
    }

    if !head.burn_after_reading {
        let text = state.db.open(stored, None).await?;
        return Ok(Content { head, text });
    }
    if !confirm_burn {
        return Err(Failure::ConfirmationRequired);
    }

    let (head, stored) = state.db.consume(id).await?;
    let text = state.db.open(stored, None).await?;
    Ok(Content { head, text })
}
