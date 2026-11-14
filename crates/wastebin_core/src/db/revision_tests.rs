//! Tests for the revisions table: the migration from `user_version` 7, the insert path and
//! deletion.

use std::num::NonZeroU32;

use rusqlite::{Connection, params};

use super::*;
use crate::revision::VersionId;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Schema of a database at `user_version` 7, as upstream migrations leave it.
const SCHEMA_V7: &str = "
    CREATE TABLE entries(
        id INTEGER PRIMARY KEY,
        burn_after_reading INTEGER,
        expires TEXT
    , uid INTEGER, data BLOB, nonce BLOB, title TEXT);
    CREATE TABLE uids (
        id INTEGER PRIMARY KEY CHECK (id = 0),
        n INTEGER
    );
    INSERT INTO uids (id, n) VALUES (0, 3);
    PRAGMA user_version = 7;
";

fn receiver() -> kanal::Receiver<Command> {
    kanal::bounded(1).1
}

fn spawn(handler: Handler) -> Database {
    let (sender, receiver) = kanal::bounded(16);
    let handler = Handler {
        receiver,
        ..handler
    };
    tokio::task::spawn_blocking(|| handler.run());
    Database {
        sender: sender.to_async(),
    }
}

async fn stored(text: &str, password: Option<&str>) -> Result<write::DatabaseEntry, Error> {
    write::Entry {
        text: text.to_string(),
        password: password.map(str::to_string),
        ..Default::default()
    }
    .compress()
    .await?
    .encrypt()
    .await
}

async fn owned(text: &str) -> Result<write::DatabaseEntry, Error> {
    let mut entry = stored(text, None).await?;
    entry.entry.uid = Some(1);
    Ok(entry)
}

fn revision_count(conn: &Connection, id: i64) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT count(*) FROM revisions WHERE entry_id = ?1",
        [id],
        |row| row.get(0),
    )
}

/// Check that `id` has exactly one revision, version 1, copied from its entry row.
fn assert_first_revision(conn: &Connection, id: i64) -> TestResult {
    assert_eq!(revision_count(conn, id)?, 1);

    let (data, title): (Vec<u8>, Option<String>) = conn.query_row(
        "SELECT data, title FROM entries WHERE id = ?1",
        [id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let revision: (i64, Vec<u8>, bool, bool) = conn.query_row(
        "SELECT r.version, r.version_id, r.parent_id IS NULL,
                r.data = e.data AND r.nonce IS e.nonce AND r.title IS e.title
                    AND r.created_at IS e.created_at
         FROM revisions r JOIN entries e ON e.id = r.entry_id WHERE r.entry_id = ?1",
        [id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;

    let version_id = VersionId::compute(None, &data, title.as_deref());
    assert_eq!(revision, (1, version_id.as_bytes().to_vec(), true, true));

    Ok(())
}

#[tokio::test]
async fn migration_backfills_first_revision() -> TestResult {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(SCHEMA_V7)?;

    let plain = stored("plain text", None).await?;
    let encrypted = stored("secret text", Some("pw")).await?;
    let burn = stored("burn text", None).await?;
    let encrypted_burn = stored("secret burn", Some("pw")).await?;

    let rows = [
        (1_i64, &plain, Some("Plain"), false),
        (2, &encrypted, Some("Secret"), false),
        (3, &burn, None, true),
        (4, &encrypted_burn, None, true),
    ];

    for (id, entry, title, burn) in rows {
        conn.execute(
            "INSERT INTO entries (id, burn_after_reading, uid, data, nonce, title) VALUES (?1, ?2, 1, ?3, ?4, ?5)",
            params![id, burn, entry.data, entry.nonce.as_ref().map(|n| n.as_slice()), title],
        )?;
    }

    let handler = Handler::with_connection(conn, receiver())?;
    let conn = &handler.conn;

    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    assert_eq!(version, 8);

    for (id, entry, _, _) in rows {
        assert_first_revision(conn, id)?;
        let (data, extension, created_at): (Vec<u8>, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT data, extension, created_at FROM entries WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
        assert_eq!(data, entry.data);
        assert_eq!((extension, created_at), (None, None));
    }

    // Rerunning the backfill and the migrations writes nothing.
    let Handler { mut conn, receiver } = handler;
    let tx = conn.transaction()?;
    assert_eq!(revision::backfill(&tx)?, 0);
    tx.commit()?;
    let handler = Handler::with_connection(conn, receiver)?;
    let total: i64 = handler
        .conn
        .query_row("SELECT count(*) FROM revisions", [], |row| row.get(0))?;
    assert_eq!(total, 4);

    // Old rows read as before, and the burn row still burns exactly once.
    let db = spawn(handler);
    let password = || Some(Password::from(b"pw".to_vec()));

    let entry = db.get(Id::from(1_i64), None).await?;
    assert!(matches!(&entry, read::Entry::Regular(data) if data.text == "plain text"));

    let entry = db.get(Id::from(2_i64), password()).await?;
    assert!(matches!(&entry, read::Entry::Encrypted(data) if data.text == "secret text"));

    let entry = db.get(Id::from(3_i64), None).await?;
    assert!(matches!(&entry, read::Entry::Burned(data) if data.text == "burn text"));
    assert!(matches!(
        db.get(Id::from(3_i64), None).await,
        Err(Error::NotFound)
    ));

    Ok(())
}

#[tokio::test]
async fn insert_writes_first_revision() -> TestResult {
    let mut handler = Handler::new(Open::Memory, receiver())?;

    let mut entry = stored("hello", None).await?;
    entry.entry.title = Some("Title".to_string());
    entry.entry.extension = Some("md".to_string());
    entry.entry.expires = NonZeroU32::new(60);
    let (plain, _) = handler.insert(entry)?;

    // Without an expiry the insert takes the other statement.
    let mut entry = stored("secret", Some("pw")).await?;
    entry.entry.extension = Some("rs".to_string());
    let (encrypted, _) = handler.insert(entry)?;

    for (id, expected) in [(plain, "md"), (encrypted, "rs")] {
        assert_first_revision(&handler.conn, id.to_i64())?;

        let (extension, created_at): (Option<String>, Option<String>) = handler.conn.query_row(
            "SELECT extension, created_at FROM entries WHERE id = ?1",
            [id.to_i64()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(extension.as_deref(), Some(expected));
        assert!(created_at.is_some());
    }

    Ok(())
}

#[tokio::test]
async fn failed_first_revision_rolls_back_the_entry() -> TestResult {
    let mut handler = Handler::new(Open::Memory, receiver())?;

    crate::revision::fail_next_insert_first();
    assert!(handler.insert(stored("lost", None).await?).is_err());

    // The entry insert shares the transaction with version 1, so neither row survives.
    for table in ["entries", "revisions"] {
        let rows: i64 =
            handler
                .conn
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get(0)
                })?;
        assert_eq!(rows, 0, "{table} kept a row after the failed insert");
    }

    // The injection fires once: the next insert works.
    let (id, _) = handler.insert(stored("kept", None).await?)?;
    assert_first_revision(&handler.conn, id.to_i64())?;

    Ok(())
}

#[tokio::test]
async fn deleting_an_entry_deletes_its_revisions() -> TestResult {
    let mut handler = Handler::new(Open::Memory, receiver())?;

    let (deleted, _) = handler.insert(owned("text").await?)?;
    let (deleted_for, _) = handler.insert(owned("text").await?)?;
    let (deleted_many, _) = handler.insert(owned("text").await?)?;
    let (expired, _) = handler.insert(owned("text").await?)?;
    let (kept, _) = handler.insert(owned("text").await?)?;

    handler.delete(deleted)?;
    handler.delete_for(deleted_for, &[1])?;
    handler.delete_many(vec![deleted_many])?;
    handler.conn.execute(
        "UPDATE entries SET expires = datetime('now', '-1 seconds') WHERE id = ?1",
        [expired.to_i64()],
    )?;
    assert_eq!(handler.purge()?, vec![expired]);

    for id in [deleted, deleted_for, deleted_many, expired] {
        assert_eq!(revision_count(&handler.conn, id.to_i64())?, 0);
    }
    assert_eq!(revision_count(&handler.conn, kept.to_i64())?, 1);

    Ok(())
}
