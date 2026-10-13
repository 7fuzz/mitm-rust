//! Refs name vaults, entries and fields rather than ids, so a rename rewrites every ref to them.

use std::sync::OnceLock;
use regex::{Captures, Regex};
use rusqlite::{params, Connection};
use super::store::{load_credential, save_credential, validate_name};
use crate::db::Db;
use super::CredentialError;

const REQUEST_COLUMNS: [&str; 8] = [
    "url",
    "headers_json",
    "params_json",
    "body_json",
    "body_raw",
    "body_form_data",
    "body_urlencoded",
    "extract_rules_json",
];

/// Maps a ref's `(vault, entry, field)` to its new names, or `None` to leave it.
pub type RefMap<'a> = &'a dyn Fn(&str, &str, &str) -> Option<(String, String, String)>;

fn ref_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        let name = "([A-Za-z0-9_-]+)";
        Regex::new(&format!(r"\{{\{{\s*(?i:\$cred):{name}/{name}\.{name}\s*\}}\}}|(?i:\$cred):{name}/{name}\.{name}"))
            .expect("valid regex")
    })
}

/// Refs from before vaults: `$cred:entry.field`.
fn legacy_ref_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        let name = "([A-Za-z0-9_-]+)";
        Regex::new(&format!(r"\{{\{{\s*(?i:\$cred):{name}\.{name}\s*\}}\}}|(?i:\$cred):{name}\.{name}")).expect("valid regex")
    })
}

/// Rewrites `{{$cred:v/e.f}}` (padded or not) and bare `$cred:v/e.f` refs that `map` renames; `None` when nothing changed.
pub fn rewrite_refs(text: &str, map: RefMap) -> Option<String> {
    if !text.contains(':') {
        return None;
    }
    let mut changed = false;
    let out = ref_regex().replace_all(text, |caps: &Captures| {
        let braced = caps.get(1).is_some();
        let at = if braced { 1 } else { 4 };
        match map(&caps[at], &caps[at + 1], &caps[at + 2]) {
            Some((vault, entry, field)) => {
                changed = true;
                if braced {
                    super::format_ref(&vault, &entry, &field)
                } else {
                    format!("$cred:{vault}/{entry}.{field}")
                }
            }
            None => caps[0].to_string(),
        }
    });
    changed.then(|| out.into_owned())
}

/// Gives pre-vault `$cred:entry.field` refs the vault they now live in.
pub(crate) fn upgrade_legacy_refs(text: &str, vault: &str) -> Option<String> {
    if !text.contains(':') {
        return None;
    }
    let mut changed = false;
    let out = legacy_ref_regex().replace_all(text, |caps: &Captures| {
        changed = true;
        match caps.get(1) {
            Some(entry) => super::format_ref(vault, entry.as_str(), &caps[2]),
            None => format!("$cred:{vault}/{}.{}", &caps[3], &caps[4]),
        }
    });
    changed.then(|| out.into_owned())
}

/// Applies `rewrite` to every environment and request text that can hold refs, in one workspace or all of them.
fn rewrite_texts(conn: &Connection, workspace_id: Option<&str>, rewrite: &dyn Fn(&str) -> Option<String>) -> Result<(), String> {
    let scope = workspace_id.unwrap_or_default();
    let envs: Vec<(String, String)> = {
        let mut stmt = conn
            .prepare("SELECT id, variables_json FROM environments WHERE ?1 = '' OR workspace_id = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([scope], |r| Ok((r.get(0)?, r.get(1)?))).map_err(|e| e.to_string())?;
        rows.filter_map(Result::ok).collect()
    };
    for (id, json) in envs {
        if let Some(json) = rewrite(&json) {
            conn.execute("UPDATE environments SET variables_json = ? WHERE id = ?", params![json, id]).map_err(|e| e.to_string())?;
        }
    }

    for column in REQUEST_COLUMNS {
        let rows: Vec<(String, String)> = {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT r.id, r.{column} FROM requests r JOIN collections c ON r.collection_id = c.id
                     WHERE (?1 = '' OR c.workspace_id = ?1) AND r.{column} LIKE '%$cred:%'"
                ))
                .map_err(|e| e.to_string())?;
            let rows = stmt.query_map([scope], |r| Ok((r.get(0)?, r.get(1)?))).map_err(|e| e.to_string())?;
            rows.filter_map(Result::ok).collect()
        };
        for (id, text) in rows {
            if let Some(text) = rewrite(&text) {
                conn.execute(&format!("UPDATE requests SET {column} = ? WHERE id = ?"), params![text, id])
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

pub(crate) fn rewrite_all(conn: &Connection, workspace_id: Option<&str>, map: RefMap) -> Result<(), String> {
    rewrite_texts(conn, workspace_id, &|text| rewrite_refs(text, map))
}

pub(crate) fn rewrite_legacy_refs(conn: &Connection, workspace_id: &str, vault: &str) -> Result<(), String> {
    rewrite_texts(conn, Some(workspace_id), &|text| upgrade_legacy_refs(text, vault))
}

/// Renames the entry and rewrites refs to it in every workspace.
pub fn rename_credential(db: &Db, id: &str, new_name: &str) -> Result<(), CredentialError> {
    validate_name(new_name)?;
    let mut conn = db.conn().map_err(|e| e.to_string())?;
    let (vault_id, vault, old_name): (String, String, String) = conn
        .query_row(
            "SELECT c.vault_id, v.name, c.name FROM credentials c JOIN vaults v ON v.id = c.vault_id WHERE c.id = ?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| e.to_string())?;
    if old_name == new_name {
        return Ok(());
    }
    let taken: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM credentials WHERE vault_id = ? AND name = ? COLLATE NOCASE AND id != ?",
            params![vault_id, new_name, id],
            |r| r.get::<_, i64>(0),
        )
        .map_err(|e| e.to_string())?
        > 0;
    if taken {
        return Err(format!("A credential named \"{new_name}\" already exists in {vault}").into());
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("UPDATE credentials SET name = ? WHERE id = ?", params![new_name, id]).map_err(|e| e.to_string())?;
    let new_name = new_name.to_string();
    rewrite_all(&tx, None, &|v, entry, field| {
        (v.eq_ignore_ascii_case(&vault) && entry.eq_ignore_ascii_case(&old_name))
            .then(|| (vault.clone(), new_name.clone(), field.to_string()))
    })?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn rename_field(db: &Db, id: &str, old_key: &str, new_key: &str) -> Result<(), CredentialError> {
    validate_name(new_key)?;
    let mut credential = load_credential(db, id)?;
    if credential.fields.iter().any(|f| f.key.eq_ignore_ascii_case(new_key) && !f.key.eq_ignore_ascii_case(old_key)) {
        return Err(format!("Field \"{new_key}\" already exists").into());
    }
    let field = credential.field_mut(old_key).ok_or_else(|| format!("Field \"{old_key}\" not found"))?;
    if field.key == new_key {
        return Ok(());
    }
    field.key = new_key.to_string();
    save_credential(db, &credential)?;

    let conn = db.conn().map_err(|e| e.to_string())?;
    let (vault, name, old_key, new_key) = (credential.vault.clone(), credential.name.clone(), old_key.to_string(), new_key.to_string());
    rewrite_all(&conn, None, &|v, entry, field| {
        (v.eq_ignore_ascii_case(&vault) && entry.eq_ignore_ascii_case(&name) && field.eq_ignore_ascii_case(&old_key))
            .then(|| (vault.clone(), name.clone(), new_key.clone()))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::store::tests::{field, temp_db, vault, workspace};
    use crate::credentials::{create_credential, rename_vault, FieldKind};
    use crate::repeater::{ExtractRuleItem, HeaderItem};
    use crate::workspace::{Environment, EnvironmentVariable, VariableVariant};

    fn rename_entry(vault: &str, entry: &str, field: &str) -> Option<(String, String, String)> {
        (vault.eq_ignore_ascii_case("DG") && entry.eq_ignore_ascii_case("dgAdmin"))
            .then(|| (vault.to_string(), "admin".to_string(), field.to_string()))
    }

    #[test]
    fn rewrites_braced_padded_and_bare_refs_only_for_the_renamed_entry() {
        let text = "{{$cred:DG/dgAdmin.password}} {{ $CRED:dg/dgadmin.token }} $cred:DG/dgAdmin.x {{$cred:DG/dgAdminX.y}} {{$cred:Other/dgAdmin.y}}";
        assert_eq!(
            rewrite_refs(text, &rename_entry).unwrap(),
            "{{$cred:DG/admin.password}} {{$cred:dg/admin.token}} $cred:DG/admin.x {{$cred:DG/dgAdminX.y}} {{$cred:Other/dgAdmin.y}}"
        );
        assert_eq!(rewrite_refs("{{$cred:Other/dgAdmin.y}}", &rename_entry), None);
    }

    #[test]
    fn legacy_refs_gain_their_vault_and_vault_refs_are_left_alone() {
        let text = "{{$cred:dgAdmin.password}} {{ $CRED:a.b }} $cred:dgAdmin.x {{$cred:DG/dgAdmin.y}} {{other}}";
        assert_eq!(
            upgrade_legacy_refs(text, "W").unwrap(),
            "{{$cred:W/dgAdmin.password}} {{$cred:W/a.b}} $cred:W/dgAdmin.x {{$cred:DG/dgAdmin.y}} {{other}}"
        );
        assert_eq!(upgrade_legacy_refs("{{$cred:DG/dgAdmin.y}}", "W"), None);
    }

    fn setup(db: &crate::db::Db, ws: &str, vault: &str) -> (Environment, crate::collections::RequestItem) {
        let cred = |field: &str| format!("{{{{$cred:{vault}/dgAdmin.{field}}}}}");
        let env = Environment {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: ws.to_string(),
            name: "dev".into(),
            is_active: true,
            variables: vec![EnvironmentVariable {
                key: "adminPassword".into(),
                value: cred("password"),
                enabled: true,
                r#type: "default".into(),
                active_index: 0,
                variants: vec![VariableVariant { name: "dev".into(), value: format!("{{{{ $cred:{vault}/dgAdmin.password }}}}") }],
            }],
            created_at_ms: 0,
            updated_at_ms: 0,
        };
        crate::workspace::save_workspace_environment_db(db, env.clone()).unwrap();
        let col = crate::collections::create_collection_db(db, ws.to_string(), None, "c".into()).unwrap();
        let mut req = crate::collections::create_request_db(db, col.id, "r".into()).unwrap();
        req.url = format!("https://x/{}", cred("password"));
        req.headers = vec![HeaderItem { id: "h".into(), key: "X-Pass".into(), value: cred("password"), enabled: true }];
        req.body_type = "json".into();
        req.body_json = Some(format!(r#"{{"p": "{}"}}"#, cred("password")));
        req.extract_rules = vec![ExtractRuleItem {
            id: "e".into(),
            r#type: "json".into(),
            expression: "token".into(),
            target_variable: format!("$cred:{vault}/dgAdmin.password"),
            enabled: true,
        }];
        crate::collections::update_request_db(db, req.clone()).unwrap();
        (env, req)
    }

    fn snapshot(db: &crate::db::Db, env: &Environment, req: &crate::collections::RequestItem) -> String {
        let conn = crate::db::open(db).unwrap();
        let vars: String = conn.query_row("SELECT variables_json FROM environments WHERE id = ?", [&env.id], |r| r.get(0)).unwrap();
        let row: String = conn
            .query_row(
                "SELECT url || headers_json || body_json || extract_rules_json FROM requests WHERE id = ?",
                [&req.id],
                |r| r.get(0),
            )
            .unwrap();
        vars + &row
    }

    #[test]
    fn renames_rewrite_refs_to_that_vault_in_every_workspace() {
        let db = temp_db();
        let (a, b, c) = (workspace(&db, "A"), workspace(&db, "B"), workspace(&db, "C"));
        let dg = vault(&db, &a, "DG");
        vault(&db, &b, "DG");
        vault(&db, &c, "Other");
        let snapshots = [setup(&db, &a, "DG"), setup(&db, &b, "DG")];
        let (other_env, other_req) = setup(&db, &c, "Other");
        let mut cred = create_credential(&db, &dg, "dgAdmin").unwrap();
        cred.fields = vec![field("password", FieldKind::Secret, "hunter2")];
        save_credential(&db, &cred).unwrap();

        rename_credential(&db, &cred.id, "admin").unwrap();
        rename_field(&db, &cred.id, "password", "pass").unwrap();
        rename_vault(&db, &dg, "Shared").unwrap();
        for (env, req) in &snapshots {
            let text = snapshot(&db, env, req);
            assert_eq!(text.matches("{{$cred:Shared/admin.pass}}").count(), 5, "{text}");
            assert!(text.contains("$cred:Shared/admin.pass\""));
            assert!(!text.contains("dgAdmin") && !text.contains("DG/"), "{text}");
        }
        assert_eq!(load_credential(&db, &cred.id).unwrap().fields[0].value, "hunter2");

        let untouched = snapshot(&db, &other_env, &other_req);
        assert_eq!(untouched.matches("Other/dgAdmin.password").count(), 6);
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn renames_reject_taken_or_invalid_names() {
        let db = temp_db();
        let ws = workspace(&db, "W");
        let dg = vault(&db, &ws, "DG");
        vault(&db, &ws, "Other");
        let a = create_credential(&db, &dg, "a").unwrap();
        create_credential(&db, &dg, "b").unwrap();
        assert!(rename_credential(&db, &a.id, "B").is_err());
        assert!(rename_credential(&db, &a.id, "a b").is_err());
        assert!(rename_vault(&db, &dg, "other").is_err());
        assert!(rename_vault(&db, &dg, "D/G").is_err());
        let mut a = load_credential(&db, &a.id).unwrap();
        a.fields = vec![field("x", FieldKind::Text, ""), field("y", FieldKind::Text, "")];
        save_credential(&db, &a).unwrap();
        assert!(rename_field(&db, &a.id, "x", "Y").is_err());
        assert!(rename_field(&db, &a.id, "x", "X").is_ok());
        let _ = std::fs::remove_file(&db);
    }
}
