-- Bodies stored once per content (sha-256 of the bytes), zstd-compressed when it pays. History rows point at them,
-- moving the existing inline bodies happens in Rust (db::bodies::migrate_history_bodies).
CREATE TABLE IF NOT EXISTS bodies (
    hash BLOB PRIMARY KEY,
    codec INTEGER NOT NULL,
    is_text INTEGER NOT NULL,
    size INTEGER NOT NULL,
    data BLOB NOT NULL
) WITHOUT ROWID;

ALTER TABLE history ADD COLUMN request_body_hash BLOB;

ALTER TABLE history ADD COLUMN response_body_hash BLOB
