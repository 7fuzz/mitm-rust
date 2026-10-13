use std::collections::BTreeMap;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;
use super::crypto;
use super::session::{self, now_ms};
use crate::db::Db;
use super::CredentialError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    #[default]
    Text,
    Secret,
    Totp,
}

impl FieldKind {
    pub const ALL: [FieldKind; 3] = [FieldKind::Text, FieldKind::Secret, FieldKind::Totp];

    pub fn label(self) -> &'static str {
        match self {
            FieldKind::Text => "text",
            FieldKind::Secret => "secret",
            FieldKind::Totp => "totp",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldMeta {
    pub key: String,
    #[serde(default)]
    pub kind: FieldKind,
    /// Written by auto-extract (tokens); only affects export.
    #[serde(default, skip_serializing_if = "is_false")]
    pub extracted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub key: String,
    pub kind: FieldKind,
    pub extracted: bool,
    pub value: String,
}

impl Field {
    pub fn meta(&self) -> FieldMeta {
        FieldMeta { key: self.key.clone(), kind: self.kind, extracted: self.extracted }
    }
}

/// An entry without its values, readable while credentials are locked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialSummary {
    pub id: String,
    pub vault_id: String,
    /// The vault's name, as refs spell it.
    pub vault: String,
    pub name: String,
    pub tags: Vec<String>,
    pub fields: Vec<FieldMeta>,
}

impl CredentialSummary {
    pub fn field(&self, key: &str) -> Option<&FieldMeta> {
        self.fields.iter().find(|f| f.key.eq_ignore_ascii_case(key))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credential {
    pub id: String,
    pub vault_id: String,
    /// The vault's name, as refs spell it; read with the entry, never written through it.
    pub vault: String,
    pub name: String,
    pub tags: Vec<String>,
    pub fields: Vec<Field>,
    pub notes: String,
    pub order_index: i64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl Credential {
    pub fn field(&self, key: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.key.eq_ignore_ascii_case(key))
    }

    pub fn field_mut(&mut self, key: &str) -> Option<&mut Field> {
        self.fields.iter_mut().find(|f| f.key.eq_ignore_ascii_case(key))
    }
}

#[derive(Serialize, Deserialize, Default)]
struct Payload {
    values: BTreeMap<String, String>,
    #[serde(default)]
    notes: String,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Vault names, entry names and field keys: `[A-Za-z0-9_-]{1,64}`, so a `{{$cred:vault/entry.field}}` ref splits cleanly.
pub fn validate_name(name: &str) -> Result<(), CredentialError> {
    let ok = (1..=64).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(format!("\"{name}\" may only use letters, digits, - and _ (up to 64)").into())
    }
}

pub const MAX_TAG_CHARS: usize = 32;

/// Trimmed, non-empty, at most `MAX_TAG_CHARS`, and each tag once ignoring case; the first spelling wins.
pub fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for tag in tags {
        let tag: String = tag.trim().chars().take(MAX_TAG_CHARS).collect();
        if !tag.is_empty() && !out.iter().any(|t| t.eq_ignore_ascii_case(&tag)) {
            out.push(tag);
        }
    }
    out
}

fn validate_fields(fields: &[Field]) -> Result<(), String> {
    for (i, field) in fields.iter().enumerate() {
        validate_name(&field.key)?;
        if fields[..i].iter().any(|f| f.key.eq_ignore_ascii_case(&field.key)) {
            return Err(format!("Field \"{}\" already exists", field.key));
        }
    }
    Ok(())
}


fn name_taken(conn: &Connection, vault_id: &str, name: &str, except_id: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM credentials WHERE vault_id = ? AND name = ? COLLATE NOCASE AND id != ?",
        params![vault_id, name, except_id],
        |_| Ok(()),
    )
    .optional()
    .map(|r| r.is_some())
    .map_err(|e| e.to_string())
}

/// Every entry in every vault, vault by vault.
pub fn list_credentials(db: &Db) -> Result<Vec<CredentialSummary>, CredentialError> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.vault_id, v.name, c.name, c.fields_json, c.tags_json FROM credentials c JOIN vaults v ON v.id = c.vault_id
             ORDER BY v.order_index, v.name, c.order_index, c.name",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            let fields_json: String = r.get(4)?;
            let tags_json: String = r.get(5)?;
            Ok(CredentialSummary {
                id: r.get(0)?,
                vault_id: r.get(1)?,
                vault: r.get(2)?,
                name: r.get(3)?,
                tags: serde_json::from_str(&tags_json).unwrap_or_default(),
                fields: serde_json::from_str(&fields_json).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

type Row = (String, String, String, String, String, String, Vec<u8>, Vec<u8>, i64, i64, i64);

fn decrypt_row(key: &crypto::DataKey, row: Row) -> Result<Credential, String> {
    let (id, vault_id, vault, name, tags_json, fields_json, nonce, ciphertext, order_index, created_at_ms, updated_at_ms) = row;
    let plain = crypto::open(key, id.as_bytes(), &nonce, &ciphertext)?;
    let payload: Payload = serde_json::from_slice(&plain).map_err(|e| e.to_string())?;
    let metas: Vec<FieldMeta> = serde_json::from_str(&fields_json).unwrap_or_default();
    let fields = metas
        .into_iter()
        .map(|m| Field {
            value: payload.values.get(&m.key).cloned().unwrap_or_default(),
            key: m.key,
            kind: m.kind,
            extracted: m.extracted,
        })
        .collect();
    let tags = serde_json::from_str(&tags_json).unwrap_or_default();
    Ok(Credential { id, vault_id, vault, name, tags, fields, notes: payload.notes, order_index, created_at_ms, updated_at_ms })
}

fn existing_key(db: &Db) -> Result<crypto::DataKey, CredentialError> {
    session::data_key(db)?.ok_or_else(|| "credentials have no data key".into())
}

const COLUMNS: &str = "id, vault_id, name, tags_json, fields_json, nonce, ciphertext, order_index, created_at_ms, updated_at_ms";
const SELECT: &str = "SELECT c.id, c.vault_id, v.name, c.name, c.tags_json, c.fields_json, c.nonce, c.ciphertext, c.order_index, \
                      c.created_at_ms, c.updated_at_ms FROM credentials c JOIN vaults v ON v.id = c.vault_id";

fn read_row(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?))
}

pub fn load_credential(db: &Db, id: &str) -> Result<Credential, CredentialError> {
    let row = db.conn().map_err(|e| e.to_string())?
        .query_row(&format!("{SELECT} WHERE c.id = ?"), [id], read_row)
        .map_err(|e| e.to_string())?;
    Ok(decrypt_row(&existing_key(db)?, row)?)
}

/// Looks an entry up by vault and entry name, ignoring case; only vaults linked to the workspace count.
pub fn find_credential(db: &Db, workspace_id: &str, vault: &str, name: &str) -> Result<Option<Credential>, CredentialError> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let Some(vault_id) = super::vault::linked_vault_id(&conn, workspace_id, vault)? else { return Ok(None) };
    let Some(row) = find_row(&conn, &vault_id, name)? else { return Ok(None) };
    Ok(Some(decrypt_row(&existing_key(db)?, row)?))
}

/// Looks an entry up by name inside one vault, ignoring case.
pub fn find_in_vault(db: &Db, vault_id: &str, name: &str) -> Result<Option<Credential>, CredentialError> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let Some(row) = find_row(&conn, vault_id, name)? else { return Ok(None) };
    Ok(Some(decrypt_row(&existing_key(db)?, row)?))
}

fn find_row(conn: &Connection, vault_id: &str, name: &str) -> Result<Option<Row>, String> {
    conn.query_row(&format!("{SELECT} WHERE c.vault_id = ? AND c.name = ? COLLATE NOCASE"), params![vault_id, name], read_row)
        .optional()
        .map_err(|e| e.to_string())
}

pub(crate) fn find_credential_conn(
    conn: &Connection,
    key: &crypto::DataKey,
    vault_id: &str,
    name: &str,
) -> Result<Option<Credential>, String> {
    find_row(conn, vault_id, name)?.map(|row| decrypt_row(key, row)).transpose()
}

/// A new, empty entry that `insert_credential_conn` can store.
pub(crate) fn new_credential(conn: &Connection, vault_id: &str, name: &str) -> Result<Credential, CredentialError> {
    validate_name(name)?;
    let vault: String = conn
        .query_row("SELECT name FROM vaults WHERE id = ?", [vault_id], |r| r.get(0))
        .map_err(|_| "Vault not found".to_string())?;
    if name_taken(conn, vault_id, name, "")? {
        return Err(format!("A credential named \"{name}\" already exists in {vault}").into());
    }
    let order_index: i64 = conn
        .query_row("SELECT COALESCE(MAX(order_index) + 1, 0) FROM credentials WHERE vault_id = ?", [vault_id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let now = now_ms();
    Ok(Credential {
        id: uuid::Uuid::new_v4().to_string(),
        vault_id: vault_id.to_string(),
        vault,
        name: name.to_string(),
        tags: Vec::new(),
        fields: Vec::new(),
        notes: String::new(),
        order_index,
        created_at_ms: now,
        updated_at_ms: now,
    })
}

pub fn create_credential(db: &Db, vault_id: &str, name: &str) -> Result<Credential, CredentialError> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let credential = new_credential(&conn, vault_id, name)?;
    write(&conn, &session::data_key_or_create(db)?, &credential, true)?;
    Ok(credential)
}

/// Saves fields, values and notes. The name only changes through `rename_credential`, which keeps refs intact.
pub fn save_credential(db: &Db, credential: &Credential) -> Result<(), CredentialError> {
    validate_fields(&credential.fields)?;
    Ok(write(&db.conn().map_err(|e| e.to_string())?, &session::data_key_or_create(db)?, credential, false)?)
}

pub(crate) fn insert_credential_conn(conn: &Connection, key: &crypto::DataKey, credential: &Credential) -> Result<(), CredentialError> {
    validate_fields(&credential.fields)?;
    Ok(write(conn, key, credential, true)?)
}

pub(crate) fn update_credential_conn(conn: &Connection, key: &crypto::DataKey, credential: &Credential) -> Result<(), CredentialError> {
    validate_fields(&credential.fields)?;
    Ok(write(conn, key, credential, false)?)
}

fn write(conn: &Connection, key: &crypto::DataKey, credential: &Credential, insert: bool) -> Result<(), String> {
    let payload = Payload {
        values: credential.fields.iter().map(|f| (f.key.clone(), f.value.clone())).collect(),
        notes: credential.notes.clone(),
    };
    let plain = Zeroizing::new(serde_json::to_vec(&payload).map_err(|e| e.to_string())?);
    let sealed = crypto::seal(key, credential.id.as_bytes(), &plain);
    let metas: Vec<FieldMeta> = credential.fields.iter().map(Field::meta).collect();
    let fields_json = serde_json::to_string(&metas).map_err(|e| e.to_string())?;
    let tags_json = serde_json::to_string(&normalize_tags(&credential.tags)).map_err(|e| e.to_string())?;
    let changed = if insert {
        conn.execute(
            &format!("INSERT INTO credentials ({COLUMNS}) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"),
            params![
                credential.id,
                credential.vault_id,
                credential.name,
                tags_json,
                fields_json,
                sealed.nonce,
                sealed.ciphertext,
                credential.order_index,
                credential.created_at_ms,
                credential.updated_at_ms,
            ],
        )
    } else {
        // The position only changes through `reorder_credentials`, so a stale copy can't undo a reorder.
        conn.execute(
            "UPDATE credentials SET tags_json = ?, fields_json = ?, nonce = ?, ciphertext = ?, updated_at_ms = ? WHERE id = ?",
            params![tags_json, fields_json, sealed.nonce, sealed.ciphertext, now_ms(), credential.id],
        )
    }
    .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Credential not found".to_string());
    }
    Ok(())
}

/// Stores an extracted value (a token) in `vault/entry.field`. A missing field is created as an extracted secret;
/// a missing entry is an error, since entries are never created implicitly.
pub fn write_extracted(db: &Db, workspace_id: &str, vault: &str, entry: &str, field: &str, value: &str) -> Result<(), CredentialError> {
    let mut credential = find_credential(db, workspace_id, vault, entry)?
        .ok_or_else(|| format!("no credential named \"{vault}/{entry}\" in this workspace's vaults"))?;
    match credential.field_mut(field) {
        Some(existing) => existing.value = value.to_string(),
        None => {
            validate_name(field)?;
            credential.fields.push(Field { key: field.to_string(), kind: FieldKind::Secret, extracted: true, value: value.to_string() });
        }
    }
    save_credential(db, &credential)
}

/// Puts the vault's entries in the order of `ids`; entries not listed keep their place after them.
pub fn reorder_credentials(db: &Db, vault_id: &str, ids: &[String]) -> Result<(), CredentialError> {
    let mut conn = db.conn().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for (index, id) in ids.iter().enumerate() {
        tx.execute("UPDATE credentials SET order_index = ? WHERE id = ? AND vault_id = ?", params![index as i64, id, vault_id])
            .map_err(|e| e.to_string())?;
    }
    tx.execute(
        "UPDATE credentials SET order_index = order_index + ? WHERE vault_id = ? AND id NOT IN (SELECT value FROM json_each(?))",
        params![ids.len() as i64, vault_id, serde_json::to_string(ids).map_err(|e| e.to_string())?],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_credential(db: &Db, id: &str) -> Result<(), CredentialError> {
    db.conn().map_err(|e| e.to_string())?
        .execute("DELETE FROM credentials WHERE id = ?", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn temp_db() -> crate::db::Db {
        let db = crate::db::Db::new(std::env::temp_dir().join(format!("mitm-creds-{}.db", uuid::Uuid::new_v4())));
        crate::db::init_database(&db, None).unwrap();
        db
    }

    pub(crate) fn workspace(db: &crate::db::Db, name: &str) -> String {
        crate::workspace::create_workspace_db(db, name.to_string(), None).unwrap().id
    }

    /// The vault named `name`, created when missing, linked to `workspace_id`.
    pub(crate) fn vault(db: &crate::db::Db, workspace_id: &str, name: &str) -> String {
        let conn = crate::db::open(db).unwrap();
        crate::credentials::ensure_linked_vault(&conn, workspace_id, name).unwrap()
    }

    pub(crate) fn field(key: &str, kind: FieldKind, value: &str) -> Field {
        Field { key: key.to_string(), kind, extracted: false, value: value.to_string() }
    }

    #[test]
    fn create_save_load_and_delete() {
        let db = temp_db();
        let ws = workspace(&db, "W");
        let dg = vault(&db, &ws, "DG");
        let mut cred = create_credential(&db, &dg, "dgAdmin").unwrap();
        assert_eq!(cred.vault, "DG");
        cred.fields = vec![field("username", FieldKind::Text, "admin@x"), field("password", FieldKind::Secret, "hunter2")];
        cred.notes = "staging only".into();
        save_credential(&db, &cred).unwrap();

        let loaded = load_credential(&db, &cred.id).unwrap();
        assert_eq!(loaded.fields, cred.fields);
        assert_eq!(loaded.notes, "staging only");
        assert_eq!(find_credential(&db, &ws, "dg", "DGADMIN").unwrap().unwrap().id, cred.id);
        assert_eq!(find_in_vault(&db, &dg, "dgadmin").unwrap().unwrap().id, cred.id);

        let listed = list_credentials(&db).unwrap();
        assert_eq!(listed[0].fields[1], FieldMeta { key: "password".into(), kind: FieldKind::Secret, extracted: false });

        let raw: Vec<u8> = crate::db::open(&db)
            .unwrap()
            .query_row("SELECT ciphertext FROM credentials", [], |r| r.get(0))
            .unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("hunter2"));

        delete_credential(&db, &cred.id).unwrap();
        assert!(list_credentials(&db).unwrap().is_empty());
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn tags_are_normalized_saved_and_listed() {
        let db = temp_db();
        let ws = workspace(&db, "W");
        let mut cred = create_credential(&db, &vault(&db, &ws, "DG"), "dgAdmin").unwrap();
        cred.tags = vec![" staging ".into(), "Admin".into(), "admin".into(), "".into(), "x".repeat(40)];
        save_credential(&db, &cred).unwrap();
        let expected = vec!["staging".to_string(), "Admin".to_string(), "x".repeat(MAX_TAG_CHARS)];
        assert_eq!(load_credential(&db, &cred.id).unwrap().tags, expected);
        assert_eq!(list_credentials(&db).unwrap()[0].tags, expected);
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn reorder_sets_the_order_and_saving_keeps_it() {
        let db = temp_db();
        let ws = workspace(&db, "W");
        let dg = vault(&db, &ws, "DG");
        let [a, _b, c] = ["a", "b", "c"].map(|name| create_credential(&db, &dg, name).unwrap());
        reorder_credentials(&db, &dg, &[c.id.clone(), a.id.clone()]).unwrap();
        save_credential(&db, &c).unwrap();
        let names: Vec<String> = list_credentials(&db).unwrap().into_iter().map(|s| s.name).collect();
        assert_eq!(names, ["c", "a", "b"]);
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn only_linked_vaults_are_found_from_a_workspace() {
        let db = temp_db();
        let (a, b) = (workspace(&db, "A"), workspace(&db, "B"));
        let dg = vault(&db, &a, "DG");
        create_credential(&db, &dg, "dgAdmin").unwrap();
        assert!(find_credential(&db, &a, "DG", "dgAdmin").unwrap().is_some());
        assert!(find_credential(&db, &b, "DG", "dgAdmin").unwrap().is_none());
        crate::credentials::link_vault(&db, &dg, &b).unwrap();
        assert!(find_credential(&db, &b, "DG", "dgAdmin").unwrap().is_some());
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn names_are_validated_and_unique_per_vault() {
        let db = temp_db();
        let ws = workspace(&db, "W");
        let (dg, other) = (vault(&db, &ws, "DG"), vault(&db, &ws, "Other"));
        for bad in ["", "dg admin", "dg.admin", "dg/admin", "dg$", &"x".repeat(65)] {
            assert!(create_credential(&db, &dg, bad).is_err(), "{bad:?}");
        }
        create_credential(&db, &dg, "dgAdmin").unwrap();
        assert!(create_credential(&db, &dg, "DGadmin").is_err());
        assert!(create_credential(&db, &other, "dgAdmin").is_ok());

        let mut cred = create_credential(&db, &dg, "two").unwrap();
        cred.fields = vec![field("token", FieldKind::Secret, "a"), field("TOKEN", FieldKind::Secret, "b")];
        assert!(save_credential(&db, &cred).is_err());
        cred.fields = vec![field("bad key", FieldKind::Text, "")];
        assert!(save_credential(&db, &cred).is_err());
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn vaults_outlive_workspaces_and_take_their_credentials_when_deleted() {
        let db = temp_db();
        let ws = workspace(&db, "W");
        let dg = vault(&db, &ws, "DG");
        create_credential(&db, &dg, "dgAdmin").unwrap();
        crate::workspace::delete_workspace_db(&db, &ws).unwrap();
        assert_eq!(list_credentials(&db).unwrap().len(), 1);
        crate::credentials::delete_vault(&db, &dg).unwrap();
        let count: i64 = crate::db::open(&db).unwrap().query_row("SELECT COUNT(*) FROM credentials", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
        let _ = std::fs::remove_file(&db);
    }
}
