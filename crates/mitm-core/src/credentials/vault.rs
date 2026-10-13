//! Vaults hold credentials. A vault can be linked to any number of workspaces, or none; a workspace's
//! `{{$cred:vault/entry.field}}` refs only reach the vaults linked to it.

use rusqlite::{params, Connection, OptionalExtension};
use super::rename::{rewrite_all, rewrite_legacy_refs};
use super::session::now_ms;
use super::store::validate_name;
use super::CredentialError;
use crate::db::Db;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vault {
    pub id: String,
    pub name: String,
    pub workspace_ids: Vec<String>,
}

impl Vault {
    pub fn is_linked(&self, workspace_id: &str) -> bool {
        self.workspace_ids.iter().any(|id| id == workspace_id)
    }
}

pub fn list_vaults(db: &Db) -> Result<Vec<Vault>, CredentialError> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let mut vaults: Vec<Vault> = {
        let mut stmt = conn.prepare("SELECT id, name FROM vaults ORDER BY order_index, name").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok(Vault { id: r.get(0)?, name: r.get(1)?, workspace_ids: Vec::new() }))
            .map_err(|e| e.to_string())?;
        rows.filter_map(Result::ok).collect()
    };
    let mut stmt = conn.prepare("SELECT vault_id, workspace_id FROM vault_workspaces").map_err(|e| e.to_string())?;
    let links = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).map_err(|e| e.to_string())?;
    for (vault_id, workspace_id) in links.filter_map(Result::ok) {
        if let Some(vault) = vaults.iter_mut().find(|v| v.id == vault_id) {
            vault.workspace_ids.push(workspace_id);
        }
    }
    Ok(vaults)
}

pub fn create_vault(db: &Db, name: &str) -> Result<Vault, CredentialError> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let id = create_vault_conn(&conn, name)?;
    Ok(Vault { id, name: name.to_string(), workspace_ids: Vec::new() })
}

pub(crate) fn create_vault_conn(conn: &Connection, name: &str) -> Result<String, CredentialError> {
    validate_name(name)?;
    if vault_id_by_name(conn, name)?.is_some() {
        return Err(format!("A vault named \"{name}\" already exists").into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_ms();
    conn.execute(
        "INSERT INTO vaults (id, name, order_index, created_at_ms, updated_at_ms)
         VALUES (?, ?, (SELECT COALESCE(MAX(order_index) + 1, 0) FROM vaults), ?, ?)",
        params![id, name, now, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

/// Renames the vault and rewrites every ref to it, in every workspace.
pub fn rename_vault(db: &Db, id: &str, new_name: &str) -> Result<(), CredentialError> {
    validate_name(new_name)?;
    let mut conn = db.conn().map_err(|e| e.to_string())?;
    let old_name: String = conn.query_row("SELECT name FROM vaults WHERE id = ?", [id], |r| r.get(0)).map_err(|e| e.to_string())?;
    if old_name == new_name {
        return Ok(());
    }
    if vault_id_by_name(&conn, new_name)?.is_some_and(|other| other != id) {
        return Err(format!("A vault named \"{new_name}\" already exists").into());
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("UPDATE vaults SET name = ?, updated_at_ms = ? WHERE id = ?", params![new_name, now_ms(), id])
        .map_err(|e| e.to_string())?;
    let new_name = new_name.to_string();
    rewrite_all(&tx, None, &|vault, entry, field| {
        vault.eq_ignore_ascii_case(&old_name).then(|| (new_name.clone(), entry.to_string(), field.to_string()))
    })?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// Deletes the vault and every credential in it; refs to it stop resolving.
pub fn delete_vault(db: &Db, id: &str) -> Result<(), CredentialError> {
    db.conn().map_err(|e| e.to_string())?.execute("DELETE FROM vaults WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn link_vault(db: &Db, vault_id: &str, workspace_id: &str) -> Result<(), CredentialError> {
    Ok(link_vault_conn(&db.conn().map_err(|e| e.to_string())?, vault_id, workspace_id)?)
}

pub(crate) fn link_vault_conn(conn: &Connection, vault_id: &str, workspace_id: &str) -> Result<(), String> {
    conn.execute("INSERT OR IGNORE INTO vault_workspaces (vault_id, workspace_id) VALUES (?, ?)", params![vault_id, workspace_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn unlink_vault(db: &Db, vault_id: &str, workspace_id: &str) -> Result<(), CredentialError> {
    db.conn()
        .map_err(|e| e.to_string())?
        .execute("DELETE FROM vault_workspaces WHERE vault_id = ? AND workspace_id = ?", params![vault_id, workspace_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn vault_id_by_name(conn: &Connection, name: &str) -> Result<Option<String>, String> {
    conn.query_row("SELECT id FROM vaults WHERE name = ? COLLATE NOCASE", [name], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())
}

/// The vault named `name`, when it's linked to the workspace.
pub(crate) fn linked_vault_id(conn: &Connection, workspace_id: &str, name: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT v.id FROM vaults v JOIN vault_workspaces vw ON vw.vault_id = v.id
         WHERE vw.workspace_id = ? AND v.name = ? COLLATE NOCASE",
        params![workspace_id, name],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// The id of the vault named `name`, created when missing, now linked to the workspace.
pub fn link_or_create_vault(db: &Db, workspace_id: &str, name: &str) -> Result<String, CredentialError> {
    ensure_linked_vault(&db.conn().map_err(|e| e.to_string())?, workspace_id, name)
}

/// The vault named `name`, created when missing, linked to the workspace.
pub(crate) fn ensure_linked_vault(conn: &Connection, workspace_id: &str, name: &str) -> Result<String, CredentialError> {
    let id = match vault_id_by_name(conn, name)? {
        Some(id) => id,
        None => create_vault_conn(conn, name)?,
    };
    link_vault_conn(conn, &id, workspace_id)?;
    Ok(id)
}

/// A vault name made from free text (a workspace name): characters a ref can't hold become `-`.
pub fn vault_name_from(text: &str) -> String {
    let mut name = String::new();
    for c in text.trim().chars() {
        let c = if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '-' };
        if !(c == '-' && name.ends_with('-')) {
            name.push(c);
        }
    }
    let name: String = name.trim_matches('-').chars().take(64).collect();
    if name.is_empty() { "vault".to_string() } else { name }
}

/// `base`, or `base-2`, `base-3`… when a vault already has that name.
pub(crate) fn free_vault_name(conn: &Connection, base: &str) -> Result<String, String> {
    if vault_id_by_name(conn, base)?.is_none() {
        return Ok(base.to_string());
    }
    for n in 2.. {
        let suffix = format!("-{n}");
        let candidate = format!("{}{suffix}", &base[..base.len().min(64 - suffix.len())]);
        if vault_id_by_name(conn, &candidate)?.is_none() {
            return Ok(candidate);
        }
    }
    unreachable!()
}

/// Migration 22: each workspace's credentials move into a vault named after it and linked to it, and the
/// workspace's `{{$cred:entry.field}}` refs gain that vault.
pub fn migrate_into_vaults(conn: &Connection) -> Result<(), String> {
    let has_workspace_column = conn
        .prepare("SELECT workspace_id FROM credentials LIMIT 0")
        .is_ok();
    if !has_workspace_column {
        return Ok(());
    }
    let workspaces: Vec<(String, String)> = {
        let mut stmt = conn
            .prepare("SELECT DISTINCT w.id, w.name FROM credentials c JOIN workspaces w ON w.id = c.workspace_id ORDER BY w.name")
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(|e| e.to_string())?;
        rows.filter_map(Result::ok).collect()
    };
    let mut vaults = Vec::new();
    for (workspace_id, workspace_name) in &workspaces {
        let name = free_vault_name(conn, &vault_name_from(workspace_name))?;
        let vault_id = create_vault_conn(conn, &name).map_err(|e| e.to_string())?;
        link_vault_conn(conn, &vault_id, workspace_id)?;
        vaults.push((workspace_id.clone(), name));
    }
    conn.execute_batch(
        "CREATE TABLE credentials_new (
             id TEXT PRIMARY KEY,
             vault_id TEXT NOT NULL REFERENCES vaults(id) ON DELETE CASCADE,
             name TEXT NOT NULL COLLATE NOCASE,
             fields_json TEXT NOT NULL DEFAULT '[]',
             nonce BLOB NOT NULL,
             ciphertext BLOB NOT NULL,
             order_index INTEGER NOT NULL DEFAULT 0,
             created_at_ms INTEGER NOT NULL,
             updated_at_ms INTEGER NOT NULL,
             UNIQUE (vault_id, name)
         );
         INSERT INTO credentials_new (id, vault_id, name, fields_json, nonce, ciphertext, order_index, created_at_ms, updated_at_ms)
             SELECT c.id, vw.vault_id, c.name, c.fields_json, c.nonce, c.ciphertext, c.order_index, c.created_at_ms, c.updated_at_ms
             FROM credentials c JOIN vault_workspaces vw ON vw.workspace_id = c.workspace_id;
         DROP TABLE credentials;
         ALTER TABLE credentials_new RENAME TO credentials;
         CREATE INDEX IF NOT EXISTS idx_credentials_vault ON credentials(vault_id, order_index);",
    )
    .map_err(|e| e.to_string())?;
    for (workspace_id, vault) in vaults {
        rewrite_legacy_refs(conn, &workspace_id, &vault)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::store::tests::{field, temp_db, vault, workspace};
    use crate::credentials::{create_credential, find_credential, save_credential, FieldKind};

    #[test]
    fn vault_names_from_free_text_fit_refs() {
        assert_eq!(vault_name_from("DG Assessment (staging)"), "DG-Assessment-staging");
        assert_eq!(vault_name_from("  ##  "), "vault");
        assert_eq!(vault_name_from(&"x".repeat(80)).len(), 64);
    }

    #[test]
    fn vaults_link_to_many_workspaces_and_names_are_unique() {
        let db = temp_db();
        let (a, b) = (workspace(&db, "A"), workspace(&db, "B"));
        let dg = create_vault(&db, "DG").unwrap();
        assert!(create_vault(&db, "dg").is_err());
        assert!(create_vault(&db, "D G").is_err());
        link_vault(&db, &dg.id, &a).unwrap();
        link_vault(&db, &dg.id, &b).unwrap();
        link_vault(&db, &dg.id, &b).unwrap();
        let listed = list_vaults(&db).unwrap();
        assert!(listed[0].is_linked(&a) && listed[0].is_linked(&b));

        crate::workspace::delete_workspace_db(&db, &a).unwrap();
        unlink_vault(&db, &dg.id, &b).unwrap();
        assert_eq!(list_vaults(&db).unwrap()[0].workspace_ids, Vec::<String>::new());
        let conn = crate::db::open(&db).unwrap();
        assert_eq!(free_vault_name(&conn, "DG").unwrap(), "DG-2");
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn migration_moves_each_workspace_into_its_own_vault_and_upgrades_refs() {
        let db = temp_db();
        let ws = workspace(&db, "DG Assessment");
        let mut entry = create_credential(&db, &vault(&db, &ws, "tmp"), "dgAdmin").unwrap();
        entry.fields = vec![field("password", FieldKind::Secret, "hunter2")];
        save_credential(&db, &entry).unwrap();
        crate::workspace::save_workspace_environment_db(&db, crate::workspace::Environment {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: ws.clone(),
            name: "dev".into(),
            is_active: true,
            variables: vec![crate::workspace::EnvironmentVariable {
                key: "adminPassword".into(),
                value: "{{$cred:dgAdmin.password}}".into(),
                enabled: true,
                r#type: "default".into(),
                active_index: 0,
                variants: Vec::new(),
            }],
            created_at_ms: 0,
            updated_at_ms: 0,
        })
        .unwrap();

        let conn = crate::db::open(&db).unwrap();
        conn.execute_batch(&format!(
            "CREATE TABLE old (id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, name TEXT NOT NULL, fields_json TEXT NOT NULL,
                 nonce BLOB NOT NULL, ciphertext BLOB NOT NULL, order_index INTEGER NOT NULL, created_at_ms INTEGER NOT NULL,
                 updated_at_ms INTEGER NOT NULL);
             INSERT INTO old SELECT id, '{ws}', name, fields_json, nonce, ciphertext, order_index, created_at_ms, updated_at_ms FROM credentials;
             DROP TABLE credentials;
             ALTER TABLE old RENAME TO credentials;
             DELETE FROM vaults;"
        ))
        .unwrap();
        migrate_into_vaults(&conn).unwrap();
        migrate_into_vaults(&conn).unwrap();
        conn.execute_batch(include_str!("../../migrations/20261013_0023_credential_tags.sql")).unwrap();

        let vaults = list_vaults(&db).unwrap();
        assert_eq!((vaults.len(), vaults[0].name.as_str()), (1, "DG-Assessment"));
        let moved = find_credential(&db, &ws, "DG-Assessment", "dgAdmin").unwrap().unwrap();
        assert_eq!(moved.field("password").unwrap().value, "hunter2");
        assert_eq!(crate::workspace::interpolate_variables(&db, &ws, "{{adminPassword}}"), "hunter2");
        let _ = std::fs::remove_file(&db);
    }
}

