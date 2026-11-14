//! The current state of a paste: its metadata and the head of its version chain.

use rusqlite::params;

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
        let (head, data, expired) = self.conn.query_row(
            "SELECT e.uid, e.title, e.extension, e.nonce IS NOT NULL,
                    COALESCE(e.burn_after_reading, 0),
                    strftime('%Y-%m-%dT%H:%M:%SZ', e.expires),
                    COALESCE(e.expires < datetime('now'), 0),
                    r.version, r.version_id, CASE WHEN ?2 THEN r.data END
             FROM entries e JOIN revisions r ON r.entry_id = e.id
             WHERE e.id = ?1
             ORDER BY r.version DESC
             LIMIT 1",
            params![id.to_i64(), with_data],
            |row| {
                let version_id = <[u8; 32]>::try_from(row.get::<_, Vec<u8>>(8)?.as_slice())
                    .map_err(|err| {
                        rusqlite::Error::FromSqlConversionFailure(
                            8,
                            rusqlite::types::Type::Blob,
                            Box::new(err),
                        )
                    })?;

                let head = Head {
                    id,
                    uid: row.get(0)?,
                    title: row.get(1)?,
                    extension: row.get(2)?,
                    encrypted: row.get(3)?,
                    burn_after_reading: row.get(4)?,
                    expires_at: row.get(5)?,
                    version: row.get(7)?,
                    version_id: VersionId::from_bytes(version_id),
                };

                Ok((head, row.get(9)?, row.get::<_, bool>(6)?))
            },
        )?;

        if expired {
            return Err(Error::NotFound);
        }

        Ok((head, data))
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use crate::db::{Database, Error, Open, read, write};
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
}
