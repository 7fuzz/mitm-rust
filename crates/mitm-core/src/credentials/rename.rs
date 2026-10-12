//! Refs name entries and fields rather than ids, so a rename rewrites every ref to them in the workspace.

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

/// Maps a ref's `(entry, field)` to its new names, or `None` to leave it.
pub type RefMap<'a> = &'a dyn Fn(&str, &str) -> Option<(String, String)>;

fn ref_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\{\{\s*(?i:\$cred):([A-Za-z0-9_-]+)\.([A-Za-z0-9_-]+)\s*\}\}|(?i:\$cred):([A-Za-z0-9_-]+)\.([A-Za-z0-9_-]+)")
            .expect("valid regex")
    })
}

/// Rewrites `{{$cred:e.f}}` (padded or not) and bare `$cred:e.f` refs that `map` renames; `None` when nothing changed.
pub fn rewrite_refs(text: &str, map: RefMap) -> Option<String> {
    if !text.contains(':') {
        return None;
    }
    let mut changed = false;
    let out = ref_regex().replace_all(text, |caps: &Captures| {
        let braced = caps.get(1).is_some();
        let (entry, field) = if braced { (&caps[1], &caps[2]) } else { (&caps[3], &caps[4]) };
        match map(entry, field) {
            Some((entry, field)) => {
                changed = true;
                if braced {
                    super::format_ref(&entry, &field)
                } else {
                    format!("$cred:{entry}.{field}")
                }
            }
            None => caps[0].to_string(),
        }
    });
    changed.then(|| out.into_owned())
}

fn rewrite_workspace(conn: &mut Connection, workspace_id: &str, map: RefMap) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let envs: Vec<(String, String)> = {
        let mut stmt = tx
            .prepare("SELECT id, variables_json FROM environments WHERE workspace_id = ?")
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([workspace_id], |r| Ok((r.get(0)?, r.get(1)?))).map_err(|e| e.to_string())?;
        rows.filter_map(Result::ok).collect()
    };
    for (id, json) in envs {
        if let Some(json) = rewrite_refs(&json, map) {
            tx.execute("UPDATE environments SET variables_json = ? WHERE id = ?", params![json, id]).map_err(|e| e.to_string())?;
        }
    }

    for column in REQUEST_COLUMNS {
        let rows: Vec<(String, String)> = {
            let mut stmt = tx
                .prepare(&format!(
                    "SELECT r.id, r.{column} FROM requests r JOIN collections c ON r.collection_id = c.id
                     WHERE c.workspace_id = ? AND r.{column} LIKE '%$cred:%'"
                ))
                .map_err(|e| e.to_string())?;
            let rows = stmt.query_map([workspace_id], |r| Ok((r.get(0)?, r.get(1)?))).map_err(|e| e.to_string())?;
            rows.filter_map(Result::ok).collect()
        };
        for (id, text) in rows {
            if let Some(text) = rewrite_refs(&text, map) {
                tx.execute(&format!("UPDATE requests SET {column} = ? WHERE id = ?"), params![text, id])
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    tx.commit().map_err(|e| e.to_string())
}

pub fn rename_credential(db: &Db, id: &str, new_name: &str) -> Result<(), CredentialError> {
    validate_name(new_name)?;
    let mut conn = db.conn().map_err(|e| e.to_string())?;
    let (workspace_id, old_name): (String, String) = conn
        .query_row("SELECT workspace_id, name FROM credentials WHERE id = ?", [id], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| e.to_string())?;
    if old_name == new_name {
        return Ok(());
    }
    let taken: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM credentials WHERE workspace_id = ? AND name = ? COLLATE NOCASE AND id != ?",
            params![workspace_id, new_name, id],
            |r| r.get::<_, i64>(0),
        )
        .map_err(|e| e.to_string())?
        > 0;
    if taken {
        return Err(format!("A credential named \"{new_name}\" already exists").into());
    }
    conn.execute("UPDATE credentials SET name = ? WHERE id = ?", params![new_name, id]).map_err(|e| e.to_string())?;
    let new_name = new_name.to_string();
    rewrite_workspace(&mut conn, &workspace_id, &|entry, field| {
        entry.eq_ignore_ascii_case(&old_name).then(|| (new_name.clone(), field.to_string()))
    })?;
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

    let mut conn = db.conn().map_err(|e| e.to_string())?;
    let (name, old_key, new_key) = (credential.name.clone(), old_key.to_string(), new_key.to_string());
    rewrite_workspace(&mut conn, &credential.workspace_id, &|entry, field| {
        (entry.eq_ignore_ascii_case(&name) && field.eq_ignore_ascii_case(&old_key)).then(|| (name.clone(), new_key.clone()))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::store::tests::{field, temp_db, workspace};
    use crate::credentials::{create_credential, FieldKind};
    use crate::repeater::{ExtractRuleItem, HeaderItem};
    use crate::workspace::{Environment, EnvironmentVariable, VariableVariant};

    fn rename_entry(entry: &str, field: &str) -> Option<(String, String)> {
        entry.eq_ignore_ascii_case("dgAdmin").then(|| ("admin".to_string(), field.to_string()))
    }

    #[test]
    fn rewrites_braced_padded_and_bare_refs_only_for_the_renamed_entry() {
        let text = "{{$cred:dgAdmin.password}} {{ $CRED:dgadmin.token }} $cred:dgAdmin.x {{$cred:dgAdminX.y}} {{$cred:other.y}}";
        assert_eq!(
            rewrite_refs(text, &rename_entry).unwrap(),
            "{{$cred:admin.password}} {{$cred:admin.token}} $cred:admin.x {{$cred:dgAdminX.y}} {{$cred:other.y}}"
        );
        assert_eq!(rewrite_refs("{{$cred:other.y}}", &rename_entry), None);
    }

    fn setup(db: &crate::db::Db, ws: &str) -> (Environment, crate::collections::RequestItem) {
        let env = Environment {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: ws.to_string(),
            name: "dev".into(),
            is_active: true,
            variables: vec![EnvironmentVariable {
                key: "adminPassword".into(),
                value: "{{$cred:dgAdmin.password}}".into(),
                enabled: true,
                r#type: "default".into(),
                active_index: 0,
                variants: vec![VariableVariant { name: "dev".into(), value: "{{ $cred:dgAdmin.password }}".into() }],
            }],
            created_at_ms: 0,
            updated_at_ms: 0,
        };
        crate::workspace::save_workspace_environment_db(db, env.clone()).unwrap();
        let col = crate::collections::create_collection_db(db, ws.to_string(), None, "c".into()).unwrap();
        let mut req = crate::collections::create_request_db(db, col.id, "r".into()).unwrap();
        req.url = "https://x/{{$cred:dgAdmin.password}}".into();
        req.headers = vec![HeaderItem { id: "h".into(), key: "X-Pass".into(), value: "{{$cred:dgAdmin.password}}".into(), enabled: true }];
        req.body_type = "json".into();
        req.body_json = Some(r#"{"p": "{{$cred:dgAdmin.password}}"}"#.into());
        req.extract_rules = vec![ExtractRuleItem {
            id: "e".into(),
            r#type: "json".into(),
            expression: "token".into(),
            target_variable: "$cred:dgAdmin.password".into(),
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
    fn renaming_an_entry_or_field_rewrites_the_workspace_and_nothing_else() {
        let db = temp_db();
        let ws = workspace(&db, "W");
        let other = workspace(&db, "Other");
        let (env, req) = setup(&db, &ws);
        let (other_env, other_req) = setup(&db, &other);
        let mut cred = create_credential(&db, &ws, "dgAdmin").unwrap();
        cred.fields = vec![field("password", FieldKind::Secret, "hunter2")];
        save_credential(&db, &cred).unwrap();

        rename_credential(&db, &cred.id, "admin").unwrap();
        let text = snapshot(&db, &env, &req);
        assert_eq!(text.matches("{{$cred:admin.password}}").count(), 5, "{text}");
        assert!(text.contains("$cred:admin.password\""));
        assert!(!text.contains("dgAdmin"));

        rename_field(&db, &cred.id, "password", "pass").unwrap();
        let text = snapshot(&db, &env, &req);
        assert!(!text.contains("admin.password"), "{text}");
        assert_eq!(text.matches("$cred:admin.pass").count(), 6);
        assert_eq!(load_credential(&db, &cred.id).unwrap().fields[0].value, "hunter2");

        let untouched = snapshot(&db, &other_env, &other_req);
        assert_eq!(untouched.matches("dgAdmin.password").count(), 6);
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn renames_reject_taken_or_invalid_names() {
        let db = temp_db();
        let ws = workspace(&db, "W");
        let a = create_credential(&db, &ws, "a").unwrap();
        create_credential(&db, &ws, "b").unwrap();
        assert!(rename_credential(&db, &a.id, "B").is_err());
        assert!(rename_credential(&db, &a.id, "a b").is_err());
        let mut a = load_credential(&db, &a.id).unwrap();
        a.fields = vec![field("x", FieldKind::Text, ""), field("y", FieldKind::Text, "")];
        save_credential(&db, &a).unwrap();
        assert!(rename_field(&db, &a.id, "x", "Y").is_err());
        assert!(rename_field(&db, &a.id, "x", "X").is_ok());
        let _ = std::fs::remove_file(&db);
    }
}
