//! The optional master password. It only wraps the data key, so setting, changing or removing it never
//! re-encrypts entries. Without one the key is stored raw and credentials are always unlocked.

use rusqlite::{params, Connection, OptionalExtension};
use zeroize::Zeroizing;
use super::crypto::{self, DataKey, KdfParams};
use super::session::{self, now_ms};
use crate::db::Db;
use super::CredentialError;

const AAD: &[u8] = b"mitm-credential-key";
const WRONG_PASSWORD: &str = "Wrong master password";

struct KeyRow {
    kdf: Option<String>,
    params: Option<String>,
    salt: Option<Vec<u8>>,
    nonce: Option<Vec<u8>>,
    wrapped: Vec<u8>,
}


fn key_row(conn: &Connection) -> Result<Option<KeyRow>, String> {
    conn.query_row("SELECT kdf, kdf_params, salt, nonce, wrapped_key FROM credential_keys WHERE id = 1", [], |r| {
        Ok(KeyRow { kdf: r.get(0)?, params: r.get(1)?, salt: r.get(2)?, nonce: r.get(3)?, wrapped: r.get(4)? })
    })
    .optional()
    .map_err(|e| e.to_string())
}

pub fn has_master_password(db: &Db) -> bool {
    db.conn().ok().and_then(|c| key_row(&c).ok().flatten()).is_some_and(|row| row.kdf.is_some())
}

pub fn is_locked(db: &Db) -> bool {
    has_master_password(db) && !session::is_cached(db)
}

fn unwrap_key(row: &KeyRow, password: &str) -> Result<DataKey, String> {
    let (Some(params), Some(salt), Some(nonce)) = (&row.params, &row.salt, &row.nonce) else {
        return crypto::key_from_slice(&row.wrapped);
    };
    let params: KdfParams = serde_json::from_str(params).map_err(|e| e.to_string())?;
    let kek = crypto::derive_key(password, salt, params)?;
    let raw = crypto::open(&kek, AAD, nonce, &row.wrapped).map_err(|_| WRONG_PASSWORD.to_string())?;
    crypto::key_from_slice(&raw)
}

pub fn unlock(db: &Db, password: &str) -> Result<(), CredentialError> {
    let row = key_row(&db.conn().map_err(|e| e.to_string())?)?.ok_or("No master password is set")?;
    let key = unwrap_key(&row, password)?;
    session::remember(db, key);
    Ok(())
}

/// Drops the data key from memory; it is zeroed on drop.
pub fn lock(db: &Db) {
    session::forget(db);
}

fn store_wrapped(conn: &Connection, key: &DataKey, password: &str) -> Result<(), String> {
    let params = KdfParams::default();
    let salt = crypto::random_bytes::<{ crypto::SALT_LEN }>();
    let kek = crypto::derive_key(password, &salt, params)?;
    let sealed = crypto::seal(&kek, AAD, key.as_slice());
    conn.execute(
        "INSERT INTO credential_keys (id, kdf, kdf_params, salt, nonce, wrapped_key, updated_at_ms) VALUES (1, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET kdf = excluded.kdf, kdf_params = excluded.kdf_params, salt = excluded.salt,
             nonce = excluded.nonce, wrapped_key = excluded.wrapped_key, updated_at_ms = excluded.updated_at_ms",
        params![crypto::KDF, serde_json::to_string(&params).map_err(|e| e.to_string())?, salt.as_slice(), sealed.nonce, sealed.ciphertext, now_ms()],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

fn store_raw(conn: &Connection, key: &DataKey) -> Result<(), String> {
    conn.execute(
        "INSERT INTO credential_keys (id, kdf, kdf_params, salt, nonce, wrapped_key, updated_at_ms) VALUES (1, NULL, NULL, NULL, NULL, ?, ?)
         ON CONFLICT(id) DO UPDATE SET kdf = NULL, kdf_params = NULL, salt = NULL, nonce = NULL,
             wrapped_key = excluded.wrapped_key, updated_at_ms = excluded.updated_at_ms",
        params![key.as_slice(), now_ms()],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// The current data key, proving `current` when a password is set.
fn current_key(conn: &Connection, current: Option<&str>) -> Result<Option<DataKey>, String> {
    let Some(row) = key_row(conn)? else { return Ok(None) };
    if row.kdf.is_none() {
        return crypto::key_from_slice(&row.wrapped).map(Some);
    }
    let current = current.ok_or_else(|| WRONG_PASSWORD.to_string())?;
    unwrap_key(&row, current).map(Some)
}

/// Sets or changes the master password. Changing needs the current one.
pub fn set_master_password(db: &Db, current: Option<&str>, new_password: &str) -> Result<(), CredentialError> {
    if new_password.is_empty() {
        return Err("The master password can't be empty".into());
    }
    let conn = db.conn().map_err(|e| e.to_string())?;
    let key = current_key(&conn, current)?.unwrap_or_else(crypto::generate_key);
    store_wrapped(&conn, &key, new_password)?;
    session::remember(db, key);
    Ok(())
}

pub fn remove_master_password(db: &Db, current: &str) -> Result<(), CredentialError> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let Some(key) = current_key(&conn, Some(current))? else { return Ok(()) };
    store_raw(&conn, &key)?;
    session::remember(db, key);
    Ok(())
}

/// For a forgotten password: every value and note is wiped, names and field keys stay, and a new raw key
/// replaces the old one.
pub fn reset_credentials(db: &Db) -> Result<(), CredentialError> {
    let mut conn = db.conn().map_err(|e| e.to_string())?;
    let key = crypto::generate_key();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let ids: Vec<String> = {
        let mut stmt = tx.prepare("SELECT id FROM credentials").map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |r| r.get(0)).map_err(|e| e.to_string())?;
        rows.filter_map(Result::ok).collect()
    };
    let empty = Zeroizing::new(br#"{"values":{},"notes":""}"#.to_vec());
    for id in ids {
        let sealed = crypto::seal(&key, id.as_bytes(), &empty);
        tx.execute("UPDATE credentials SET nonce = ?, ciphertext = ? WHERE id = ?", params![sealed.nonce, sealed.ciphertext, id])
            .map_err(|e| e.to_string())?;
    }
    store_raw(&tx, &key)?;
    tx.commit().map_err(|e| e.to_string())?;
    session::remember(db, key);
    Ok(())
}

/// `Err(LOCKED)` when a master password is set and not entered this session.
pub fn ensure_unlocked(db: &Db) -> Result<(), CredentialError> {
    if is_locked(db) {
        Err(CredentialError::Locked)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::store::tests::{field, temp_db, workspace};
    use crate::credentials::{create_credential, find_credential, load_credential, resolve, save_credential, FieldKind, Resolve};

    fn with_entry() -> (crate::db::Db, String, String) {
        let db = temp_db();
        let ws = workspace(&db, "W");
        let mut cred = create_credential(&db, &ws, "dgAdmin").unwrap();
        cred.fields = vec![field("password", FieldKind::Secret, "hunter2")];
        cred.notes = "n".into();
        save_credential(&db, &cred).unwrap();
        (db, ws, cred.id)
    }

    #[test]
    fn set_change_and_remove_keep_entries_readable() {
        let (db, ws, id) = with_entry();
        assert!(!has_master_password(&db));
        set_master_password(&db, None, "one").unwrap();
        assert!(has_master_password(&db) && !is_locked(&db));

        lock(&db);
        assert!(is_locked(&db));
        assert_eq!(resolve(&db, &ws, "dgAdmin", "password"), Resolve::Locked);
        assert_eq!(load_credential(&db, &id).unwrap_err(), CredentialError::Locked);
        assert_eq!(unlock(&db, "wrong").unwrap_err(), CredentialError::from(WRONG_PASSWORD));
        unlock(&db, "one").unwrap();
        assert_eq!(load_credential(&db, &id).unwrap().fields[0].value, "hunter2");

        assert!(set_master_password(&db, Some("wrong"), "two").is_err());
        assert!(set_master_password(&db, None, "two").is_err());
        set_master_password(&db, Some("one"), "two").unwrap();
        lock(&db);
        assert!(unlock(&db, "one").is_err());
        unlock(&db, "two").unwrap();

        assert!(remove_master_password(&db, "one").is_err());
        remove_master_password(&db, "two").unwrap();
        lock(&db);
        assert!(!is_locked(&db));
        assert_eq!(find_credential(&db, &ws, "dgAdmin").unwrap().unwrap().fields[0].value, "hunter2");
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn a_password_set_before_any_credential_still_works() {
        let db = temp_db();
        let ws = workspace(&db, "W");
        set_master_password(&db, None, "pw").unwrap();
        create_credential(&db, &ws, "a").unwrap();
        lock(&db);
        assert_eq!(create_credential(&db, &ws, "b").unwrap_err(), CredentialError::Locked);
        unlock(&db, "pw").unwrap();
        create_credential(&db, &ws, "b").unwrap();
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn reset_wipes_values_but_keeps_names_and_field_keys() {
        let (db, ws, id) = with_entry();
        set_master_password(&db, None, "forgotten").unwrap();
        lock(&db);
        reset_credentials(&db).unwrap();
        assert!(!has_master_password(&db));
        let cred = load_credential(&db, &id).unwrap();
        assert_eq!(cred.name, "dgAdmin");
        assert_eq!((cred.fields[0].key.as_str(), cred.fields[0].value.as_str()), ("password", ""));
        assert_eq!(cred.notes, "");
        assert!(find_credential(&db, &ws, "dgadmin").unwrap().is_some());
        let _ = std::fs::remove_file(&db);
    }
}
