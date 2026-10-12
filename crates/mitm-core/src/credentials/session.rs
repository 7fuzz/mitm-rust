//! The unlocked data key per database. Interpolation is free functions taking `db_path`, so the key lives
//! in a process-wide registry keyed by that path rather than being threaded through every call.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use rusqlite::{params, Connection, OptionalExtension};
use super::crypto::{self, DataKey};
use super::LOCKED;

fn sessions() -> &'static Mutex<HashMap<PathBuf, DataKey>> {
    static SESSIONS: OnceLock<Mutex<HashMap<PathBuf, DataKey>>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cached(db_path: &Path) -> Option<DataKey> {
    sessions().lock().ok()?.get(db_path).cloned()
}

fn remember(db_path: &Path, key: DataKey) {
    if let Ok(mut map) = sessions().lock() {
        map.insert(db_path.to_path_buf(), key);
    }
}

#[cfg(test)]
pub(crate) fn forget(db_path: &Path) {
    if let Ok(mut map) = sessions().lock() {
        map.remove(db_path);
    }
}

pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// The data key, `None` before the first credential is saved, or `Err(LOCKED)`.
pub(crate) fn data_key(db_path: &Path) -> Result<Option<DataKey>, String> {
    if let Some(key) = cached(db_path) {
        return Ok(Some(key));
    }
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let row: Option<(Option<String>, Vec<u8>)> = conn
        .query_row("SELECT kdf, wrapped_key FROM credential_keys WHERE id = 1", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()
        .map_err(|e| e.to_string())?;
    match row {
        None => Ok(None),
        Some((Some(_), _)) => Err(LOCKED.to_string()),
        Some((None, raw)) => {
            let key = crypto::key_from_slice(&raw)?;
            remember(db_path, key.clone());
            Ok(Some(key))
        }
    }
}

/// The data key, created and stored raw on first use.
pub(crate) fn data_key_or_create(db_path: &Path) -> Result<DataKey, String> {
    if let Some(key) = data_key(db_path)? {
        return Ok(key);
    }
    let key = crypto::generate_key();
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let inserted = conn
        .execute(
            "INSERT OR IGNORE INTO credential_keys (id, kdf, wrapped_key, updated_at_ms) VALUES (1, NULL, ?, ?)",
            params![key.as_slice(), now_ms()],
        )
        .map_err(|e| e.to_string())?;
    if inserted == 0 {
        // Another writer stored a key first; use theirs.
        return data_key(db_path)?.ok_or_else(|| "data key missing".to_string());
    }
    remember(db_path, key.clone());
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::store::tests::temp_db;

    #[test]
    fn key_is_created_once_and_survives_a_fresh_session() {
        let db = temp_db();
        assert!(data_key(&db).unwrap().is_none());
        let key = data_key_or_create(&db).unwrap();
        assert_eq!(data_key_or_create(&db).unwrap(), key);
        forget(&db);
        assert_eq!(data_key(&db).unwrap().unwrap(), key);
        let _ = std::fs::remove_file(&db);
    }
}
