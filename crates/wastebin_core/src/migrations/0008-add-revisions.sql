ALTER TABLE entries ADD COLUMN extension TEXT;
ALTER TABLE entries ADD COLUMN created_at TEXT;

-- One row per stored version of an entry. `data`, `nonce` and `title` hold the bytes exactly as
-- `entries` stores them (compressed, and ciphertext when encrypted).
CREATE TABLE revisions (
    entry_id INTEGER NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    version_id BLOB NOT NULL CHECK (length(version_id) = 32),
    parent_id BLOB CHECK (parent_id IS NULL OR length(parent_id) = 32),
    data BLOB NOT NULL,
    nonce BLOB,
    title TEXT,
    created_at TEXT,
    PRIMARY KEY (entry_id, version),
    UNIQUE (entry_id, version_id)
);

-- A trigger rather than a foreign key: SQLite enforces foreign keys only on connections that turn
-- them on, and the trigger also covers deletes made by other tools.
CREATE TRIGGER entries_delete_revisions AFTER DELETE ON entries
BEGIN
    DELETE FROM revisions WHERE entry_id = OLD.id;
END;
