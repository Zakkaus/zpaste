//! Tests of [`super`]: heads, verification without consumption, consumption and the owner list.
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
    let created_at = head.created_at.as_deref().ok_or("no creation time")?;
    assert_eq!(created_at.len(), "2026-11-14T13:00:00Z".len());
    assert!(created_at < expires_at, "{created_at} {expires_at}");

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

#[test]
fn expired_paste_is_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler = handler()?;
    let id = insert_at(&mut handler, 1, None)?;
    assert!(handler.head(id, false).is_ok());

    expire(&handler, id)?;
    assert!(matches!(handler.head(id, false), Err(Error::NotFound)));

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

fn expire(handler: &Handler, id: Id) -> Result<(), Box<dyn std::error::Error>> {
    handler.conn.execute(
        "UPDATE entries SET expires = datetime('now', '-1 second') WHERE id = ?1",
        rusqlite::params![id.to_i64()],
    )?;
    Ok(())
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

#[test]
fn list_skips_expired_and_refuses_them_as_marker() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler = handler()?;
    let expiring = insert_at(&mut handler, 1, Some("2026-01-02 00:00:00"))?;
    let kept = insert_at(&mut handler, 1, Some("2026-01-01 00:00:00"))?;
    expire(&handler, expiring)?;

    let page = handler.list_for(&[1], None, 20)?;
    assert_eq!(
        page.heads.iter().map(|head| head.id).collect::<Vec<_>>(),
        [kept]
    );
    assert!(matches!(
        handler.list_for(&[1], Some(expiring), 20),
        Err(Error::InvalidMarker)
    ));

    Ok(())
}

/// Two handlers on their own connections to one database file race to consume each paste:
/// exactly one of them gets it.
#[test]
fn consumers_on_two_connections_get_a_paste_once() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!(
        "wastebin-consume-{}-{}.sqlite",
        std::process::id(),
        rand::random::<u64>()
    ));
    let result = race_consumers(&path);
    for suffix in ["", "-journal"] {
        let mut file = path.clone().into_os_string();
        file.push(suffix);
        let _ = std::fs::remove_file(file);
    }
    result
}

fn race_consumers(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let open = || -> Result<Handler, Box<dyn std::error::Error>> {
        let (_sender, receiver) = kanal::bounded(1);
        Ok(Handler::with_connection(
            rusqlite::Connection::open(path)?,
            receiver,
        )?)
    };
    let mut first = open()?;
    let mut second = open()?;

    for _ in 0..100 {
        let id = insert_at(&mut first, 1, None)?;
        let start = std::sync::Barrier::new(2);
        let (a, b) = std::thread::scope(|scope| {
            let a = scope.spawn(|| {
                start.wait();
                first.consume(id)
            });
            let b = scope.spawn(|| {
                start.wait();
                second.consume(id)
            });
            (a.join(), b.join())
        });
        let won = [a.map_err(|_| "panic")?, b.map_err(|_| "panic")?]
            .into_iter()
            .filter_map(|result| match result {
                Ok(_) => Some(Ok(())),
                Err(Error::NotFound) => None,
                Err(err) => Some(Err(err)),
            })
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(won.len(), 1);
    }

    Ok(())
}
