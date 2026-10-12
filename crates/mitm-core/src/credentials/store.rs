use std::collections::BTreeMap;
use std::path::Path;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;
use super::crypto;
use super::session::{self, now_ms};

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
    pub workspace_id: String,
    pub name: String,
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
    pub workspace_id: String,
    pub name: String,
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

/// Entry names and field keys: `[A-Za-z0-9_-]{1,64}`, so a `{{$cred:entry.field}}` ref splits on its one dot.
pub fn validate_name(name: &str) -> Result<(), String> {
    let ok = (1..=64).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(format!("\"{name}\" may only use letters, digits, - and _ (up to 64)"))
    }
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

fn open(db_path: &Path) -> Result<Connection, String> {
    crate::db::open(db_path).map_err(|e| e.to_string())
}

fn name_taken(conn: &Connection, workspace_id: &str, name: &str, except_id: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM credentials WHERE workspace_id = ? AND name = ? COLLATE NOCASE AND id != ?",
        params![workspace_id, name, except_id],
        |_| Ok(()),
    )
    .optional()
    .map(|r| r.is_some())
    .map_err(|e| e.to_string())
}

pub fn list_credentials(db_path: &Path, workspace_id: &str) -> Result<Vec<CredentialSummary>, String> {
    let conn = open(db_path)?;
    let mut stmt = conn
        .prepare("SELECT id, workspace_id, name, fields_json FROM credentials WHERE workspace_id = ? ORDER BY order_index, name")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([workspace_id], |r| {
            let fields_json: String = r.get(3)?;
            Ok(CredentialSummary {
                id: r.get(0)?,
                workspace_id: r.get(1)?,
                name: r.get(2)?,
                fields: serde_json::from_str(&fields_json).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

type Row = (String, String, String, String, Vec<u8>, Vec<u8>, i64, i64, i64);

fn decrypt_row(key: &crypto::DataKey, row: Row) -> Result<Credential, String> {
    let (id, workspace_id, name, fields_json, nonce, ciphertext, order_index, created_at_ms, updated_at_ms) = row;
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
    Ok(Credential { id, workspace_id, name, fields, notes: payload.notes, order_index, created_at_ms, updated_at_ms })
}

fn existing_key(db_path: &Path) -> Result<crypto::DataKey, String> {
    session::data_key(db_path)?.ok_or_else(|| "credentials have no data key".to_string())
}

const COLUMNS: &str = "id, workspace_id, name, fields_json, nonce, ciphertext, order_index, created_at_ms, updated_at_ms";

fn read_row(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?))
}

pub fn load_credential(db_path: &Path, id: &str) -> Result<Credential, String> {
    let row = open(db_path)?
        .query_row(&format!("SELECT {COLUMNS} FROM credentials WHERE id = ?"), [id], read_row)
        .map_err(|e| e.to_string())?;
    decrypt_row(&existing_key(db_path)?, row)
}

/// Looks an entry up by name, ignoring case.
pub fn find_credential(db_path: &Path, workspace_id: &str, name: &str) -> Result<Option<Credential>, String> {
    let conn = open(db_path)?;
    let Some(row) = find_row(&conn, workspace_id, name)? else { return Ok(None) };
    decrypt_row(&existing_key(db_path)?, row).map(Some)
}

fn find_row(conn: &Connection, workspace_id: &str, name: &str) -> Result<Option<Row>, String> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM credentials WHERE workspace_id = ? AND name = ? COLLATE NOCASE"),
        params![workspace_id, name],
        read_row,
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub(crate) fn find_credential_conn(
    conn: &Connection,
    key: &crypto::DataKey,
    workspace_id: &str,
    name: &str,
) -> Result<Option<Credential>, String> {
    find_row(conn, workspace_id, name)?.map(|row| decrypt_row(key, row)).transpose()
}

/// A new, empty entry that `insert_credential_conn` can store.
pub(crate) fn new_credential(conn: &Connection, workspace_id: &str, name: &str) -> Result<Credential, String> {
    validate_name(name)?;
    if name_taken(conn, workspace_id, name, "")? {
        return Err(format!("A credential named \"{name}\" already exists"));
    }
    let order_index: i64 = conn
        .query_row("SELECT COALESCE(MAX(order_index) + 1, 0) FROM credentials WHERE workspace_id = ?", [workspace_id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let now = now_ms();
    Ok(Credential {
        id: uuid::Uuid::new_v4().to_string(),
        workspace_id: workspace_id.to_string(),
        name: name.to_string(),
        fields: Vec::new(),
        notes: String::new(),
        order_index,
        created_at_ms: now,
        updated_at_ms: now,
    })
}

pub fn create_credential(db_path: &Path, workspace_id: &str, name: &str) -> Result<Credential, String> {
    let conn = open(db_path)?;
    let credential = new_credential(&conn, workspace_id, name)?;
    write(&conn, &session::data_key_or_create(db_path)?, &credential, true)?;
    Ok(credential)
}

/// Saves fields, values and notes. The name only changes through `rename_credential`, which keeps refs intact.
pub fn save_credential(db_path: &Path, credential: &Credential) -> Result<(), String> {
    validate_fields(&credential.fields)?;
    write(&open(db_path)?, &session::data_key_or_create(db_path)?, credential, false)
}

pub(crate) fn insert_credential_conn(conn: &Connection, key: &crypto::DataKey, credential: &Credential) -> Result<(), String> {
    validate_fields(&credential.fields)?;
    write(conn, key, credential, true)
}

pub(crate) fn update_credential_conn(conn: &Connection, key: &crypto::DataKey, credential: &Credential) -> Result<(), String> {
    validate_fields(&credential.fields)?;
    write(conn, key, credential, false)
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
    let changed = if insert {
        conn.execute(
            &format!("INSERT INTO credentials ({COLUMNS}) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"),
            params![
                credential.id,
                credential.workspace_id,
                credential.name,
                fields_json,
                sealed.nonce,
                sealed.ciphertext,
                credential.order_index,
                credential.created_at_ms,
                credential.updated_at_ms,
            ],
        )
    } else {
        conn.execute(
            "UPDATE credentials SET fields_json = ?, nonce = ?, ciphertext = ?, order_index = ?, updated_at_ms = ? WHERE id = ?",
            params![fields_json, sealed.nonce, sealed.ciphertext, credential.order_index, now_ms(), credential.id],
        )
    }
    .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Credential not found".to_string());
    }
    Ok(())
}

/// Stores an extracted value (a token) in `entry.field`. A missing field is created as an extracted secret;
/// a missing entry is an error, since entries are never created implicitly.
pub fn write_extracted(db_path: &Path, workspace_id: &str, entry: &str, field: &str, value: &str) -> Result<(), String> {
    let mut credential = find_credential(db_path, workspace_id, entry)?.ok_or_else(|| format!("no credential named \"{entry}\""))?;
    match credential.field_mut(field) {
        Some(existing) => existing.value = value.to_string(),
        None => {
            validate_name(field)?;
            credential.fields.push(Field { key: field.to_string(), kind: FieldKind::Secret, extracted: true, value: value.to_string() });
        }
    }
    save_credential(db_path, &credential)
}

pub fn delete_credential(db_path: &Path, id: &str) -> Result<(), String> {
    open(db_path)?
        .execute("DELETE FROM credentials WHERE id = ?", [id])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn temp_db() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("mitm-creds-{}.db", uuid::Uuid::new_v4()));
        crate::db::init_database(&path, None).unwrap();
        path
    }

    pub(crate) fn workspace(db: &std::path::PathBuf, name: &str) -> String {
        crate::workspace::create_workspace_db(db, name.to_string(), None).unwrap().id
    }

    pub(crate) fn field(key: &str, kind: FieldKind, value: &str) -> Field {
        Field { key: key.to_string(), kind, extracted: false, value: value.to_string() }
    }

    #[test]
    fn create_save_load_and_delete() {
        let db = temp_db();
        let ws = workspace(&db, "W");
        let mut cred = create_credential(&db, &ws, "dgAdmin").unwrap();
        cred.fields = vec![field("username", FieldKind::Text, "admin@x"), field("password", FieldKind::Secret, "hunter2")];
        cred.notes = "staging only".into();
        save_credential(&db, &cred).unwrap();

        let loaded = load_credential(&db, &cred.id).unwrap();
        assert_eq!(loaded.fields, cred.fields);
        assert_eq!(loaded.notes, "staging only");
        assert_eq!(find_credential(&db, &ws, "DGADMIN").unwrap().unwrap().id, cred.id);

        let listed = list_credentials(&db, &ws).unwrap();
        assert_eq!(listed[0].fields[1], FieldMeta { key: "password".into(), kind: FieldKind::Secret, extracted: false });

        let raw: Vec<u8> = crate::db::open(&db)
            .unwrap()
            .query_row("SELECT ciphertext FROM credentials", [], |r| r.get(0))
            .unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("hunter2"));

        delete_credential(&db, &cred.id).unwrap();
        assert!(list_credentials(&db, &ws).unwrap().is_empty());
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn names_are_validated_and_unique_per_workspace() {
        let db = temp_db();
        let ws = workspace(&db, "W");
        let other = workspace(&db, "Other");
        for bad in ["", "dg admin", "dg.admin", "dg$", &"x".repeat(65)] {
            assert!(create_credential(&db, &ws, bad).is_err(), "{bad:?}");
        }
        create_credential(&db, &ws, "dgAdmin").unwrap();
        assert!(create_credential(&db, &ws, "DGadmin").is_err());
        assert!(create_credential(&db, &other, "dgAdmin").is_ok());

        let mut cred = create_credential(&db, &ws, "two").unwrap();
        cred.fields = vec![field("token", FieldKind::Secret, "a"), field("TOKEN", FieldKind::Secret, "b")];
        assert!(save_credential(&db, &cred).is_err());
        cred.fields = vec![field("bad key", FieldKind::Text, "")];
        assert!(save_credential(&db, &cred).is_err());
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn deleting_a_workspace_deletes_its_credentials() {
        let db = temp_db();
        let ws = workspace(&db, "W");
        create_credential(&db, &ws, "dgAdmin").unwrap();
        crate::workspace::delete_workspace_db(&db, &ws).unwrap();
        let count: i64 = crate::db::open(&db).unwrap().query_row("SELECT COUNT(*) FROM credentials", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
        let _ = std::fs::remove_file(&db);
    }
}
