//! The current state of a paste: its metadata and the head of its version chain.

use chacha20poly1305::XNonce;
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter};

use super::{Error, Handler, write};
use crate::id::Id;
use crate::revision::VersionId;

/// Metadata of a paste and its current (head) version, as the `/v1` API shows it.
#[derive(Debug)]
pub struct Head {
    /// Identifier
    pub id: Id,
    /// User identifier that inserted the entry; `None` for pastes without an owner
    pub uid: Option<i64>,
    /// Title as stored, also for encrypted pastes
    pub title: Option<String>,
    /// Extension stored at creation; `None` for pastes from before the column
    pub extension: Option<String>,
    /// Whether the content is encrypted
    pub encrypted: bool,
    /// Whether the paste is deleted after its first read
    pub burn_after_reading: bool,
    /// Expiry in RFC 3339, UTC; `None` when the paste never expires
    pub expires_at: Option<String>,
    /// Depth of the head version in the version chain, starting at 1
    pub version: u32,
    /// Id of the head version
    pub version_id: VersionId,
}

/// The stored bytes of one version: compressed, and ciphertext when `nonce` is set.
#[derive(Debug)]
pub struct Stored {
    pub data: Vec<u8>,
    pub nonce: Option<XNonce>,
}

/// One page of [`super::Database::list_for`].
#[derive(Debug)]
pub struct ListPage {
    /// At most the requested number of pastes, newest first.
    pub heads: Vec<Head>,
    /// Whether more pastes follow the last one.
    pub more: bool,
}

/// Columns every head query selects, `e` being `entries` and `r` the head revision; `?2` asks for
/// the stored bytes.
const HEAD_COLUMNS: &str = "e.id, e.uid, e.title, e.extension, e.nonce IS NOT NULL,
    COALESCE(e.burn_after_reading, 0), strftime('%Y-%m-%dT%H:%M:%SZ', e.expires),
    COALESCE(e.expires < datetime('now'), 0), r.version, r.version_id,
    CASE WHEN ?2 THEN r.data END, CASE WHEN ?2 THEN r.nonce END";

fn blob_error(
    index: usize,
    err: impl std::error::Error + Send + Sync + 'static,
) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Blob, Box::new(err))
}

/// A row of [`HEAD_COLUMNS`]: the head, the stored bytes when asked for, and whether it expired.
fn head_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(Head, Option<Stored>, bool)> {
    let version_id = <[u8; 32]>::try_from(row.get::<_, Vec<u8>>(9)?.as_slice())
        .map_err(|err| blob_error(9, err))?;
    let nonce = row
        .get::<_, Option<Vec<u8>>>(11)?
        .map(|nonce| XNonce::try_from(nonce.as_slice()).map_err(|err| blob_error(11, err)))
        .transpose()?;
    let stored = row
        .get::<_, Option<Vec<u8>>>(10)?
        .map(|data| Stored { data, nonce });

    let head = Head {
        id: Id::from(row.get::<_, i64>(0)?),
        uid: row.get(1)?,
        title: row.get(2)?,
        extension: row.get(3)?,
        encrypted: row.get(4)?,
        burn_after_reading: row.get(5)?,
        expires_at: row.get(6)?,
        version: row.get(8)?,
        version_id: VersionId::from_bytes(version_id),
    };

    Ok((head, stored, row.get(7)?))
}

impl Handler {
    /// Insert `entry` and read back its [`Head`]. The handler runs one command at a time, so no
    /// other command sees the paste between the two steps.
    pub(super) fn create(&mut self, entry: write::DatabaseEntry) -> Result<Head, Error> {
        let (id, _entry) = self.insert(entry)?;
        Ok(self.head(id, false)?.0)
    }

    /// Read the [`Head`] of `id` and, with `with_data`, the stored bytes of the head version.
    /// An expired paste is [`Error::NotFound`]. Unlike [`super::Database::get`], this never
    /// deletes anything, so it never consumes a burn-after-reading paste.
    pub(super) fn head(&self, id: Id, with_data: bool) -> super::HeadResult {
        let (head, stored, expired) = self.conn.query_row(
            &format!(
                "SELECT {HEAD_COLUMNS}
                 FROM entries e JOIN revisions r ON r.entry_id = e.id
                 WHERE e.id = ?1
                 ORDER BY r.version DESC
                 LIMIT 1"
            ),
            params![id.to_i64(), with_data],
            head_from_row,
        )?;

        if expired {
            return Err(Error::NotFound);
        }

        Ok((head, stored))
    }

    /// Read the head of `id` with its stored bytes and delete the paste, as one command. The
    /// handler runs one command at a time, so of two consumers of the same paste exactly one gets
    /// it and the other [`Error::NotFound`].
    pub(super) fn consume(&self, id: Id) -> Result<(Head, Stored), Error> {
        let (head, stored) = self.head(id, true)?;
        self.conn
            .execute("DELETE FROM entries WHERE id = ?1", params![id.to_i64()])?;
        Ok((head, stored.ok_or(Error::NotFound)?))
    }

    /// One page of the unexpired pastes owned by any of `uids`, newest first by
    /// `(created_at, id)`, pastes without `created_at` last. `marker` is the last id of the
    /// previous page; one that is not an unexpired paste of `uids` is [`Error::InvalidMarker`].
    pub(super) fn list_for(
        &self,
        uids: &[i64],
        marker: Option<Id>,
        limit: usize,
    ) -> Result<ListPage, Error> {
        const UNEXPIRED: &str = "NOT COALESCE(e.expires < datetime('now'), 0)";

        if uids.is_empty() {
            return Ok(ListPage {
                heads: Vec::new(),
                more: false,
            });
        }

        // `?1` is the marker and `?2` the `with_data` flag of `HEAD_COLUMNS`; the uids start at
        // `?3`.
        let owned = (3..uids.len() + 3)
            .map(|n| format!("?{n}"))
            .collect::<Vec<_>>()
            .join(",");
        let mut params = vec![Value::Null, Value::from(false)];
        params.extend(uids.iter().copied().map(Value::from));

        let after = match marker {
            None => String::from("1"),
            Some(marker) => {
                params[0] = Value::from(marker.to_i64());
                let created_at: Option<String> = self
                    .conn
                    .query_row(
                        &format!(
                            "SELECT e.created_at FROM entries e
                             WHERE e.id = ?1 AND e.uid IN ({owned}) AND {UNEXPIRED}"
                        ),
                        params_from_iter(&params),
                        |row| row.get(0),
                    )
                    .map_err(|err| match err {
                        rusqlite::Error::QueryReturnedNoRows => Error::InvalidMarker,
                        err => err.into(),
                    })?;

                match created_at {
                    // Rows after the marker: older, as old with a smaller id, or undated.
                    Some(created_at) => {
                        params.push(Value::from(created_at));
                        let at = params.len();
                        format!(
                            "(e.created_at IS NULL OR e.created_at < ?{at}
                              OR (e.created_at = ?{at} AND e.id < ?1))"
                        )
                    }
                    None => String::from("(e.created_at IS NULL AND e.id < ?1)"),
                }
            }
        };

        let fetch = limit + 1;
        let mut heads = self
            .conn
            .prepare(&format!(
                "SELECT {HEAD_COLUMNS}
                 FROM entries e JOIN revisions r ON r.entry_id = e.id
                     AND r.version = (SELECT MAX(version) FROM revisions WHERE entry_id = e.id)
                 WHERE e.uid IN ({owned}) AND {UNEXPIRED} AND {after}
                 ORDER BY e.created_at DESC NULLS LAST, e.id DESC
                 LIMIT {fetch}"
            ))?
            .query_map(params_from_iter(&params), |row| {
                head_from_row(row).map(|(head, _, _)| head)
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let more = heads.len() > limit;
        heads.truncate(limit);
        Ok(ListPage { heads, more })
    }
}

#[cfg(test)]
mod tests;
