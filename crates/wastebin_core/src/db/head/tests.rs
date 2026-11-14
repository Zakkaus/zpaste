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
async fn list_skips_expired_and_refuses_them_as_marker() -> Result<(), Box<dyn std::error::Error>> {
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
