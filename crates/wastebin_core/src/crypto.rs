use std::num::NonZeroUsize;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use rand::RngExt;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::spawn_blocking;

use crate::env;

static CONFIG: LazyLock<argon2::Config> = LazyLock::new(|| argon2::Config {
    variant: argon2::Variant::Argon2i,
    version: argon2::Version::Version13,
    mem_cost: 65536,
    time_cost: 10,
    lanes: 4,
    thread_mode: argon2::ThreadMode::Parallel,
    secret: &[],
    ad: &[],
    hash_length: 32,
});

static SALT: LazyLock<String> = LazyLock::new(env::password_hash_salt);

/// Encryption or decryption errors.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to hash with argon2: {0}")]
    Argon2(#[from] argon2::Error),
    #[error("failed to encrypt")]
    ChaCha20Poly1305Encrypt,
    #[error("failed to decrypt")]
    ChaCha20Poly1305Decrypt,
    #[error("join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("too many password key derivations in progress")]
    Busy,
}

/// Admission control for password key derivation. One derivation holds 64 MiB and four threads
/// for its whole run, so at most `concurrency` run at once across every clone of a limit. A
/// caller that finds no free slot within `wait` gets [`Error::Busy`] and derives nothing.
#[derive(Clone, Debug)]
pub struct KdfLimit {
    slots: Arc<Semaphore>,
    wait: Duration,
}

/// A slot of a [`KdfLimit`], held until the derivation it admitted has finished.
#[derive(Debug)]
pub struct KdfPermit(#[allow(dead_code, reason = "held for its drop")] OwnedSemaphorePermit);

impl KdfLimit {
    /// Concurrent derivations when the operator sets none: 128 MiB of Argon2 memory.
    pub const DEFAULT_CONCURRENCY: NonZeroUsize = NonZeroUsize::new(2).expect("2 is non-zero");
    /// How long a request waits for a slot before it is refused.
    pub const DEFAULT_WAIT: Duration = Duration::from_secs(1);

    #[must_use]
    pub fn new(concurrency: NonZeroUsize, wait: Duration) -> Self {
        Self {
            slots: Arc::new(Semaphore::new(concurrency.get())),
            wait,
        }
    }

    /// Wait up to the configured time for a free slot.
    pub async fn acquire(&self) -> Result<KdfPermit, Error> {
        match tokio::time::timeout(self.wait, self.slots.clone().acquire_owned()).await {
            Ok(Ok(permit)) => Ok(KdfPermit(permit)),
            Ok(Err(_)) | Err(_) => Err(Error::Busy),
        }
    }
}

impl Default for KdfLimit {
    fn default() -> Self {
        Self::new(Self::DEFAULT_CONCURRENCY, Self::DEFAULT_WAIT)
    }
}

/// Encrypted data item.
pub(crate) struct Encrypted {
    /// Encrypted ciphertext.
    pub ciphertext: Vec<u8>,
    /// Nonce used for encryption.
    pub nonce: XNonce,
}

pub struct Password(Vec<u8>);

/// Plaintext bytes to be encrypted.
pub(crate) struct Plaintext(Vec<u8>);

impl From<Vec<u8>> for Password {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl From<Vec<u8>> for Plaintext {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

fn cipher_from(password: &[u8]) -> Result<XChaCha20Poly1305, Error> {
    let key = argon2::hash_raw(password, SALT.as_bytes(), &CONFIG)?;
    let key = Key::try_from(key.as_slice()).map_err(|_| Error::ChaCha20Poly1305Encrypt)?;
    Ok(XChaCha20Poly1305::new(&key))
}

impl Plaintext {
    /// Consume and encrypt plaintext into [`Encrypted`] using `password`. The blocking task owns
    /// `permit`, so the slot stays taken until the derivation ends, even if the caller is dropped.
    pub async fn encrypt(self, password: Password, permit: KdfPermit) -> Result<Encrypted, Error> {
        spawn_blocking(move || {
            let _permit = permit;
            let cipher = cipher_from(&password.0)?;
            let nonce = XNonce::from(rand::rng().random::<[u8; 24]>());
            let ciphertext = cipher
                .encrypt(&nonce, self.0.as_ref())
                .map_err(|_| Error::ChaCha20Poly1305Encrypt)?;

            Ok(Encrypted::new(ciphertext, nonce))
        })
        .await?
    }
}

impl Encrypted {
    /// Create new [`Encrypted`] item from `ciphertext` and `nonce`.
    pub fn new(ciphertext: Vec<u8>, nonce: XNonce) -> Self {
        Self { ciphertext, nonce }
    }

    /// Decrypt into bytes using `password`, holding `permit` like [`Plaintext::encrypt`].
    pub async fn decrypt(self, password: Password, permit: KdfPermit) -> Result<Vec<u8>, Error> {
        spawn_blocking(move || {
            let _permit = permit;
            let cipher = cipher_from(&password.0)?;
            let nonce = XNonce::try_from(self.nonce.as_slice())
                .map_err(|_| Error::ChaCha20Poly1305Decrypt)?;
            let plaintext = cipher
                .decrypt(&nonce, self.ciphertext.as_ref())
                .map_err(|_| Error::ChaCha20Poly1305Decrypt)?;
            Ok(plaintext)
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn roundtrip() {
        let password = "secret".to_string();
        let plaintext = "encrypt me".to_string();
        let limit = KdfLimit::default();
        let encrypted = Plaintext::from(plaintext.as_bytes().to_vec())
            .encrypt(
                Password::from(password.as_bytes().to_vec()),
                limit.acquire().await.unwrap(),
            )
            .await
            .unwrap();
        let decrypted = encrypted
            .decrypt(
                Password::from(password.as_bytes().to_vec()),
                limit.acquire().await.unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(decrypted, plaintext.as_bytes());
    }

    #[tokio::test]
    async fn limit_refuses_when_every_slot_is_taken() {
        let limit = KdfLimit::new(NonZeroUsize::MIN, Duration::ZERO);
        let held = limit.clone().acquire().await.unwrap();
        assert!(matches!(limit.acquire().await, Err(Error::Busy)));

        drop(held);
        assert!(limit.acquire().await.is_ok());
    }
}
