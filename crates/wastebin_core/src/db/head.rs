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
mod tests {
    use std::num::NonZeroU32;

    use crate::crypto::Password;
    use crate::db::{Database, Error, Handler, Open, read, write};
    use crate::id::Id;
    use crate::revision::VersionId;

    fn new_db() -> Result<Database, Box<dyn std::error::Error>> {
        let (db, handler) = Database::new(Open::Memory)?;
        tokio::spawn(handler);
        Ok(db)
    }

    #[tokio::test]
    async fn create_returns_first_version() -> Result<(), Box<dyn std::error::Error>> {
        let db = new_db()?;

        let head = db
            .create(write::Entry {
                text: "hello".to_string(),
                extension: Some("md".to_string()),
                title: Some("Title".to_string()),
                uid: Some(7),
                expires: NonZeroU32::new(3600),
                ..Default::default()
            })
            .await?;

        assert_eq!(head.uid, Some(7));
        assert_eq!(head.extension.as_deref(), Some("md"));
        assert_eq!(head.title.as_deref(), Some("Title"));
        assert!(!head.encrypted);
        assert!(!head.burn_after_reading);
        assert_eq!(head.version, 1);
        let expires_at = head.expires_at.as_deref().ok_or("no expiry")?;
        assert_eq!(expires_at.len(), "2026-11-21T13:00:00Z".len());
        assert!(expires_at.ends_with('Z'));

        let (again, data) = db.head_with_data(head.id).await?;
        assert_eq!(again.version_id, head.version_id);
        assert_eq!(
            head.version_id,
            VersionId::compute(None, &data, Some("Title"))
        );
        assert_eq!(read::decompress(data).await?, "hello");

        Ok(())
    }

    #[tokio::test]
    async fn never_expiring_paste_has_no_expiry() -> Result<(), Box<dyn std::error::Error>> {
        let db = new_db()?;

        let head = db
            .create(write::Entry {
                text: "x".to_string(),
                password: Some("secret".to_string()),
                burn_after_reading: Some(true),
                ..Default::default()
            })
            .await?;

        assert!(head.expires_at.is_none());
        assert!(head.encrypted);
        assert!(head.burn_after_reading);
        assert!(head.extension.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn head_never_consumes_burn_paste() -> Result<(), Box<dyn std::error::Error>> {
        let db = new_db()?;

        let head = db
            .create(write::Entry {
                text: "secret".to_string(),
                burn_after_reading: Some(true),
                ..Default::default()
            })
            .await?;

        db.head(head.id).await?;
        db.head_with_data(head.id).await?;
        assert_eq!(db.get(head.id, None).await?.unwrap_inner().text, "secret");
        assert!(matches!(db.head(head.id).await, Err(Error::NotFound)));

        Ok(())
    }

    #[tokio::test]
    async fn expired_paste_is_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let db = new_db()?;

        let head = db
            .create(write::Entry {
                text: "x".to_string(),
                expires: NonZeroU32::new(1),
                ..Default::default()
            })
            .await?;

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        assert!(matches!(db.head(head.id).await, Err(Error::NotFound)));

        Ok(())
    }

    fn password(value: &str) -> Option<Password> {
        Some(Password::from(value.as_bytes().to_vec()))
    }

    #[tokio::test]
    async fn open_verifies_without_consuming() -> Result<(), Box<dyn std::error::Error>> {
        let db = new_db()?;
        let head = db
            .create(write::Entry {
                text: "secret".to_string(),
                password: Some("right".to_string()),
                burn_after_reading: Some(true),
                ..Default::default()
            })
            .await?;

        let (_, stored) = db.head_with_stored(head.id).await?;
        assert!(matches!(
            db.open(stored, password("wrong")).await,
            Err(Error::Crypto(crate::crypto::Error::ChaCha20Poly1305Decrypt))
        ));
        let (_, stored) = db.head_with_stored(head.id).await?;
        assert!(matches!(
            db.open(stored, None).await,
            Err(Error::NoPassword)
        ));
        let (_, stored) = db.head_with_stored(head.id).await?;
        assert_eq!(db.open(stored, password("right")).await?, "secret");

        // Still there: only `consume` deletes.
        let (consumed, _) = db.consume(head.id).await?;
        assert_eq!(consumed.version_id, head.version_id);
        assert!(matches!(db.head(head.id).await, Err(Error::NotFound)));

        Ok(())
    }

    #[tokio::test]
    async fn concurrent_consumers_get_a_paste_once() -> Result<(), Box<dyn std::error::Error>> {
        let db = new_db()?;
        let head = db
            .create(write::Entry {
                text: "once".to_string(),
                burn_after_reading: Some(true),
                ..Default::default()
            })
            .await?;

        let consumers = (0..16).map(|_| {
            let db = db.clone();
            tokio::spawn(async move { db.consume(head.id).await })
        });
        let mut won = 0;
        for consumer in consumers {
            match consumer.await? {
                Ok((_, stored)) => {
                    won += 1;
                    assert_eq!(read::decompress(stored.data).await?, "once");
                }
                Err(Error::NotFound) => {}
                Err(err) => return Err(err.into()),
            }
        }
        assert_eq!(won, 1);

        Ok(())
    }

    /// Creates pastes for `uid` and returns their ids in creation order.
    async fn create_many(
        db: &Database,
        uid: i64,
        count: usize,
    ) -> Result<Vec<Id>, Box<dyn std::error::Error>> {
        let mut ids = Vec::new();
        for n in 0..count {
            let head = db
                .create(write::Entry {
                    text: format!("paste {n}"),
                    uid: Some(uid),
                    ..Default::default()
                })
                .await?;
            ids.push(head.id);
        }
        Ok(ids)
    }

    /// A handler on a fresh in-memory database, driven directly so a test can set `created_at`.
    fn handler() -> Result<Handler, Box<dyn std::error::Error>> {
        let (_sender, receiver) = kanal::bounded(1);
        Ok(Handler::with_connection(
            rusqlite::Connection::open_in_memory()?,
            receiver,
        )?)
    }

    fn insert_at(
        handler: &mut Handler,
        uid: i64,
        created_at: Option<&str>,
    ) -> Result<Id, Box<dyn std::error::Error>> {
        let (id, _) = handler.insert(write::DatabaseEntry {
            entry: write::Entry {
                uid: Some(uid),
                ..Default::default()
            },
            data: Vec::new(),
            nonce: None,
        })?;
        handler.conn.execute(
            "UPDATE entries SET created_at = ?1 WHERE id = ?2",
            rusqlite::params![created_at, id.to_i64()],
        )?;
        Ok(id)
    }

    #[test]
    fn list_pages_by_owner_newest_first() -> Result<(), Box<dyn std::error::Error>> {
        let mut handler = handler()?;
        let newest = insert_at(&mut handler, 1, Some("2026-01-02 00:00:00"))?;
        let mut same_second = vec![
            insert_at(&mut handler, 1, Some("2026-01-01 00:00:00"))?,
            insert_at(&mut handler, 1, Some("2026-01-01 00:00:00"))?,
        ];
        let mut undated = vec![
            insert_at(&mut handler, 1, None)?,
            insert_at(&mut handler, 1, None)?,
        ];
        let theirs = insert_at(&mut handler, 2, Some("2026-01-03 00:00:00"))?;

        // Within one `created_at` the larger id comes first; undated pastes come last.
        same_second.sort_by_key(|id| std::cmp::Reverse(id.to_i64()));
        undated.sort_by_key(|id| std::cmp::Reverse(id.to_i64()));
        let expected = [vec![newest], same_second, undated].concat();

        let mut listed = Vec::new();
        let mut marker = None;
        loop {
            let page = handler.list_for(&[1], marker, 2)?;
            assert!(page.heads.len() <= 2);
            listed.extend(page.heads.iter().map(|head| head.id));
            marker = listed.last().copied();
            if !page.more {
                break;
            }
        }
        assert_eq!(listed, expected);

        // A cookie with both identities lists both.
        assert_eq!(handler.list_for(&[1, 2], None, 100)?.heads.len(), 6);

        // Another identity's paste and an unknown id are the same refusal.
        for marker in [theirs, Id::from(12_345_i64)] {
            assert!(matches!(
                handler.list_for(&[1], Some(marker), 2),
                Err(Error::InvalidMarker)
            ));
        }

        Ok(())
    }

    #[tokio::test]
    async fn list_skips_expired_and_refuses_them_as_marker()
    -> Result<(), Box<dyn std::error::Error>> {
        let db = new_db()?;
        let expiring = db
            .create(write::Entry {
                text: "x".to_string(),
                uid: Some(1),
                expires: NonZeroU32::new(1),
                ..Default::default()
            })
            .await?;
        let kept = create_many(&db, 1, 1).await?;

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let page = db.list_for(&[1], None, 20).await?;
        assert_eq!(
            page.heads.iter().map(|head| head.id).collect::<Vec<_>>(),
            kept
        );
        assert!(matches!(
            db.list_for(&[1], Some(expiring.id), 20).await,
            Err(Error::InvalidMarker)
        ));

        Ok(())
    }
}
