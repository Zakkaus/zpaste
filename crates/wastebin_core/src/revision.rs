//! Version ids and the revision rows stored next to each entry.

use std::fmt;

#[cfg(test)]
use std::cell::Cell;

use rusqlite::{Transaction, params};
use sha2::{Digest, Sha256};

/// Identifier of one stored version: a SHA-256 hash chain over the stored bytes.
///
/// ```text
/// version_id = sha256( parent_id                  32 bytes; no bytes for the first version
///                    || u64be(len(data)) || data  data as stored: compressed, ciphertext when encrypted
///                    || u8(has_title) || u32be(len(title)) || title (UTF-8; length 0 without title) )
/// ```
///
/// Hashing the stored bytes means a version id is computed without the password and reveals
/// nothing about the plaintext of an encrypted paste.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VersionId([u8; 32]);

impl VersionId {
    /// Compute the id of a version with `parent` (`None` for the first version), the stored
    /// `data` and the optional `title`.
    #[must_use]
    pub fn compute(parent: Option<&VersionId>, data: &[u8], title: Option<&str>) -> Self {
        let mut hasher = Sha256::new();

        if let Some(parent) = parent {
            hasher.update(parent.0);
        }

        hasher.update((data.len() as u64).to_be_bytes());
        hasher.update(data);

        let title = title.map(str::as_bytes);
        // SQLite caps a value at 1e9 bytes, so a stored title always fits.
        let title_len = u32::try_from(title.map_or(0, <[u8]>::len)).expect("title below 4 GiB");
        hasher.update([u8::from(title.is_some())]);
        hasher.update(title_len.to_be_bytes());
        hasher.update(title.unwrap_or_default());

        Self(hasher.finalize().into())
    }

    /// Raw 32 bytes as stored in the database.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Id from the 32 bytes stored in the database.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl fmt::Display for VersionId {
    /// 64 lowercase hex digits.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}

#[cfg(test)]
thread_local! {
    /// Makes the next `insert_first` on this thread fail once, so a test can prove that the entry
    /// insert and version 1 share one transaction.
    static FAIL_NEXT_INSERT_FIRST: Cell<bool> = const { Cell::new(false) };
}

/// Make the next `insert_first` on this thread return an error.
#[cfg(test)]
pub(crate) fn fail_next_insert_first() {
    FAIL_NEXT_INSERT_FIRST.set(true);
}

/// Write version 1 of entry `entry_id` from the bytes `entries` holds for it.
///
/// The row copies `data`, `nonce`, `title` and `created_at` from `entries` inside `tx`, so the
/// revision always matches what the entry stores; `data` and `title` are only hashed here.
pub(crate) fn insert_first(
    tx: &Transaction,
    entry_id: i64,
    data: &[u8],
    title: Option<&str>,
) -> rusqlite::Result<VersionId> {
    #[cfg(test)]
    if FAIL_NEXT_INSERT_FIRST.replace(false) {
        return Err(rusqlite::Error::InvalidQuery);
    }

    let version_id = VersionId::compute(None, data, title);

    tx.execute(
        "INSERT INTO revisions (entry_id, version, version_id, parent_id, data, nonce, title, created_at)
         SELECT id, 1, ?2, NULL, data, nonce, title, created_at FROM entries WHERE id = ?1",
        params![entry_id, version_id.as_bytes()],
    )?;

    Ok(version_id)
}

/// Write version 1 for every entry that has no revision yet and return how many were written.
///
/// Reads only the stored bytes: nothing is decrypted, and burn-after-reading entries stay unread.
/// Entries that already have a revision are skipped, so a second run writes nothing.
pub(crate) fn backfill(tx: &Transaction) -> rusqlite::Result<usize> {
    let mut select = tx.prepare(
        "SELECT id FROM entries e WHERE NOT EXISTS (SELECT 1 FROM revisions r WHERE r.entry_id = e.id)",
    )?;
    let ids = select
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut read = tx.prepare("SELECT data, title FROM entries WHERE id = ?1")?;

    for &id in &ids {
        let (data, title) = read.query_row([id], |row| {
            Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Option<String>>(1)?))
        })?;
        insert_first(tx, id, &data, title.as_deref())?;
    }

    Ok(ids.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(parent: Option<&VersionId>, data: &[u8], title: Option<&str>) -> String {
        VersionId::compute(parent, data, title).to_string()
    }

    /// Vectors computed independently with Python's `hashlib` from the layout above.
    #[test]
    fn version_id_vectors() {
        assert_eq!(
            hex(None, b"", None),
            "dd46c3eebb1884ff3b5258c0a2fc9398e560a29e0780d4b53869b6254aa46a96"
        );
        assert_eq!(
            hex(None, b"hello", None),
            "aabdf59e83749f903fc72d0cb4ac9590ff67107e89e54b187b1cf1ed8d6168ff"
        );
        assert_eq!(
            hex(None, b"hello", Some("")),
            "982242bd25d8ad55b01add5cfb759f975d151f62f55b91694daedfbecd22944f"
        );
        assert_eq!(
            hex(None, b"", Some("")),
            "a19775b89ef31f9d8a7fcaa4f052afee0b2a693c90764e9d7a845200ef714b06"
        );
        assert_eq!(
            hex(None, b"\x00\x01\xff", Some("標題")),
            "46f51cc67422a72e333868dc4652b7290d9f149c1acea21ca060920568a984e2"
        );

        let first = VersionId::compute(None, b"hello", Some("Title"));
        assert_eq!(
            first.to_string(),
            "ff7f4550b859c9e9afb48515b4c9fcad93633b77cfea814722c6f4af068716d0"
        );
        // Same content under a parent gets a new id.
        assert_eq!(
            hex(Some(&first), b"hello", Some("Title")),
            "86b5c2b936a30f0ebbec675df70d9bd2da883a4af4c1dc3b8c0ce2f962a318a5"
        );
    }
}
