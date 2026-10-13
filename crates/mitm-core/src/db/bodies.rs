//! Request and response bodies, stored once per content (SHA-256 of the bytes) and zstd-compressed when it pays.
//! Callers keep the in-memory form used everywhere else: text as is, binary as `base64:<data>`.

use base64::Engine;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

const CODEC_RAW: i64 = 0;
const CODEC_ZSTD: i64 = 1;
const ZSTD_LEVEL: i32 = 3;

/// Stores `body` unless it's already there; `None` for an empty body.
pub fn put(conn: &Connection, body: &str) -> Result<Option<Vec<u8>>, String> {
    if body.is_empty() {
        return Ok(None);
    }
    let (bytes, is_text) = to_bytes(body);
    let hash = Sha256::digest(&bytes).to_vec();
    let known = conn
        .query_row("SELECT 1 FROM bodies WHERE hash = ?", [&hash], |_| Ok(()))
        .optional()
        .map_err(|e| e.to_string())?
        .is_some();
    if !known {
        let compressed = zstd::bulk::compress(&bytes, ZSTD_LEVEL).map_err(|e| e.to_string())?;
        // Already-compressed content (images, fonts, archives) is kept raw rather than paying to decode it.
        let (codec, data) = if compressed.len() * 10 < bytes.len() * 9 { (CODEC_ZSTD, compressed) } else { (CODEC_RAW, bytes.clone()) };
        conn.execute(
            "INSERT OR IGNORE INTO bodies (hash, codec, is_text, size, data) VALUES (?, ?, ?, ?, ?)",
            params![hash, codec, is_text, bytes.len() as i64, data],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(Some(hash))
}

/// The body stored under `hash`, in the form `put` was given; empty for `None` or a missing body.
pub fn get(conn: &Connection, hash: Option<&[u8]>) -> Result<String, String> {
    let Some(hash) = hash else { return Ok(String::new()) };
    let row: Option<(i64, bool, i64, Vec<u8>)> = conn
        .query_row("SELECT codec, is_text, size, data FROM bodies WHERE hash = ?", [hash], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .optional()
        .map_err(|e| e.to_string())?;
    let Some((codec, is_text, size, data)) = row else { return Ok(String::new()) };
    let bytes = match codec {
        CODEC_ZSTD => zstd::bulk::decompress(&data, size.max(0) as usize).map_err(|e| e.to_string())?,
        _ => data,
    };
    Ok(from_bytes(bytes, is_text))
}

/// `body`, or a short note in its place when it's over `max_bytes` (0: no limit).
pub fn within_limit(body: &str, max_bytes: u64) -> std::borrow::Cow<'_, str> {
    let size = match body.strip_prefix("base64:") {
        Some(b64) => b64.len() as u64 / 4 * 3,
        None => body.len() as u64,
    };
    if max_bytes == 0 || size <= max_bytes {
        return std::borrow::Cow::Borrowed(body);
    }
    let mb = |bytes: u64| bytes as f64 / (1024.0 * 1024.0);
    std::borrow::Cow::Owned(format!("[Body not stored: {:.1} MB, over the {:.0} MB limit]", mb(size), mb(max_bytes)))
}

/// Deletes bodies no history row points to any more.
pub fn collect_garbage(conn: &Connection) -> Result<usize, String> {
    conn.execute(
        "DELETE FROM bodies WHERE hash NOT IN (
             SELECT request_body_hash FROM history WHERE request_body_hash IS NOT NULL
             UNION SELECT response_body_hash FROM history WHERE response_body_hash IS NOT NULL)",
        [],
    )
    .map_err(|e| e.to_string())
}

fn to_bytes(body: &str) -> (Vec<u8>, bool) {
    body.strip_prefix("base64:")
        .and_then(|b64| base64::engine::general_purpose::STANDARD.decode(b64).ok())
        .map_or_else(|| (body.as_bytes().to_vec(), true), |bytes| (bytes, false))
}

fn from_bytes(bytes: Vec<u8>, is_text: bool) -> String {
    if is_text {
        String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
    } else {
        format!("base64:{}", base64::engine::general_purpose::STANDARD.encode(bytes))
    }
}

/// Migration 24: history bodies move from inline text into the store, a batch of rows at a time.
pub fn migrate_history_bodies(conn: &Connection) -> Result<(), String> {
    const BATCH: i64 = 200;
    let mut after = i64::MIN;
    loop {
        let rows: Vec<(i64, String, String)> = {
            let mut stmt = conn
                .prepare(
                    "SELECT id, request_body, response_body FROM history
                     WHERE id > ? AND (request_body != '' OR response_body != '') ORDER BY id LIMIT ?",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![after, BATCH], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(|e| e.to_string())?;
            rows.collect::<Result<_, _>>().map_err(|e| e.to_string())?
        };
        let Some(last) = rows.last() else { return Ok(()) };
        after = last.0;
        for (id, request, response) in rows {
            conn.execute(
                "UPDATE history SET request_body_hash = ?, response_body_hash = ?, request_body = '', response_body = '' WHERE id = ?",
                params![put(conn, &request)?, put(conn, &response)?, id],
            )
            .map_err(|e| e.to_string())?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> (crate::db::Db, Connection) {
        let db = crate::db::Db::new(std::env::temp_dir().join(format!("mitm-bodies-{}.db", uuid::Uuid::new_v4())));
        crate::db::init_database(&db, None).unwrap();
        let conn = db.conn().unwrap();
        (db, conn)
    }

    #[test]
    fn bodies_round_trip_in_their_original_form() {
        let (db, conn) = conn();
        let json = format!(r#"{{"items": [{}]}}"#, vec![r#"{"name": "a long repeated value"}"#; 200].join(","));
        let png = format!("base64:{}", base64::engine::general_purpose::STANDARD.encode([0x89, b'P', b'N', b'G', 0, 255, 7]));
        for body in [json.as_str(), "plain text", png.as_str(), "base64:not valid base64!", "ünïcode ✓"] {
            let hash = put(&conn, body).unwrap();
            assert_eq!(get(&conn, hash.as_deref()).unwrap(), body);
        }
        assert_eq!(put(&conn, "").unwrap(), None);
        assert_eq!(get(&conn, None).unwrap(), "");

        let stored: i64 = conn
            .query_row("SELECT LENGTH(data) FROM bodies WHERE hash = ?", [Sha256::digest(json.as_bytes()).to_vec()], |r| r.get(0))
            .unwrap();
        assert!(stored * 10 < json.len() as i64, "repetitive JSON compresses: {stored} of {}", json.len());
        let _ = std::fs::remove_file(db.path());
    }

    #[test]
    fn bodies_over_the_limit_become_a_note() {
        assert_eq!(within_limit("small", 0), "small");
        assert_eq!(within_limit("small", 10), "small");
        let big = "x".repeat(3 * 1024 * 1024);
        assert_eq!(within_limit(&big, 1024 * 1024), "[Body not stored: 3.0 MB, over the 1 MB limit]");
        let binary = format!("base64:{}", "A".repeat(4 * 1024 * 1024));
        assert_eq!(within_limit(&binary, 2 * 1024 * 1024), "[Body not stored: 3.0 MB, over the 2 MB limit]");
    }

    #[test]
    fn identical_bodies_are_stored_once_and_collected_when_unused() {
        let (db, conn) = conn();
        let insert = |id: i64, body: &str| {
            let hash = put(&conn, body).unwrap();
            conn.execute(
                "INSERT INTO history (id, method, url, host, status_code, request_headers, response_headers, request_body, response_body, phase, response_body_hash)
                 VALUES (?, 'GET', 'http://a', 'a', 200, '[]', '[]', '', '', 'done', ?)",
                params![id, hash],
            )
            .unwrap();
        };
        insert(1, "same bundle");
        insert(2, "same bundle");
        insert(3, "other");
        let count = || conn.query_row("SELECT COUNT(*) FROM bodies", [], |r| r.get::<_, i64>(0)).unwrap();
        assert_eq!(count(), 2);
        conn.execute("DELETE FROM history WHERE id IN (1, 3)", []).unwrap();
        assert_eq!(collect_garbage(&conn).unwrap(), 1);
        assert_eq!(count(), 1);
        let _ = std::fs::remove_file(db.path());
    }

    #[test]
    fn migration_moves_inline_bodies_and_reads_give_the_same_strings() {
        let (db, conn) = conn();
        let png = format!("base64:{}", base64::engine::general_purpose::STANDARD.encode([0x89, b'P', b'N', b'G']));
        conn.execute(
            "INSERT INTO history (id, method, url, host, status_code, request_headers, response_headers, request_body, response_body, phase)
             VALUES (7, 'POST', 'http://a', 'a', 200, '[]', '[]', '{\"q\":1}', ?, 'done')",
            [&png],
        )
        .unwrap();
        migrate_history_bodies(&conn).unwrap();

        let inline: (String, String) = conn.query_row("SELECT request_body, response_body FROM history WHERE id = 7", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(inline, (String::new(), String::new()));
        let detail = crate::db::get_history_detail(&db, 7).unwrap().unwrap();
        assert_eq!((detail.request_body.as_str(), detail.response_body.as_str()), (r#"{"q":1}"#, png.as_str()));
        let full = crate::db::get_history_by_ids(&db, &[7]).unwrap();
        assert_eq!(full[0].response_body, png);
        let _ = std::fs::remove_file(db.path());
    }
}

