use std::collections::{HashMap, HashSet};
use rusqlite::params;

use crate::collections::{get_collections_db, CollectionTreeItem, RequestItem};
use crate::http::body::{MultipartBodySpec, MultipartFieldSpec, UrlEncodedParamSpec};
use crate::repeater::{HeaderItem, ParamItem};
use crate::workspace::get_workspace_environments_db;
use super::file::{
    make_ref, Body, ExportOptions, CollectionEntry, EnvironmentEntry, ExtractEntry, ExtractSource, FormField, FormFieldType, KeyValue,
    RequestEntry, VariableEntry, WorkspaceFile, WorkspaceInfo,
};
use crate::db::Db;

/// The workspace as a `mitm-workspace` file. Credential values go in only when sealed with `credentials_passphrase`.
pub fn export_workspace_json_db(
    db: &Db,
    workspace_id: &str,
    options: ExportOptions,
    credentials_passphrase: Option<&str>,
) -> Result<String, String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let (name, description): (String, Option<String>) = conn
        .query_row("SELECT name, description FROM workspaces WHERE id = ?", params![workspace_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|e| format!("Workspace with ID '{}' not found: {}", workspace_id, e))?;

    let mut file = WorkspaceFile::new(WorkspaceInfo { name, description: non_empty(description) });
    file.environments = get_workspace_environments_db(db, workspace_id)?
        .iter()
        .map(|env| EnvironmentEntry {
            name: env.name.clone(),
            active: env.is_active,
            variables: env.variables.iter().map(|v| VariableEntry::from_variable(v, options)).collect(),
        })
        .collect();

    let tree = get_collections_db(db, workspace_id)?;
    let refs = step_refs(&tree);
    file.collections = tree.iter().map(|node| collection_entry(node, &refs)).collect();
    (file.vaults, file.credentials_sealed) =
        crate::credentials::export_credentials(db, workspace_id, credentials_passphrase, options.include_auto)?;

    serde_json::to_string_pretty(&file).map_err(|e| format!("Failed to serialize workspace: {}", e))
}

pub fn export_workspace_file_db(
    db: &Db,
    workspace_id: &str,
    destination_path: &str,
    options: ExportOptions,
    credentials_passphrase: Option<&str>,
) -> Result<(), String> {
    let json = export_workspace_json_db(db, workspace_id, options, credentials_passphrase)?;
    let target = std::path::Path::new(destination_path);
    if let Some(parent) = target.parent().filter(|p| !p.exists()) {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create destination directory: {}", e))?;
    }
    std::fs::write(target, json).map_err(|e| format!("Failed to write workspace file to '{}': {}", destination_path, e))
}

/// Refs for the requests some other request runs before or after it; the rest don't need one.
fn step_refs(tree: &[CollectionTreeItem]) -> HashMap<String, String> {
    fn walk<'a>(nodes: &'a [CollectionTreeItem], out: &mut Vec<&'a RequestItem>) {
        for node in nodes {
            out.extend(&node.requests);
            walk(&node.children, out);
        }
    }
    let mut requests = Vec::new();
    walk(tree, &mut requests);
    let targets: HashSet<&str> =
        requests.iter().flat_map(|r| r.pre_requests.iter().chain(&r.post_requests)).map(String::as_str).collect();
    let mut taken = HashSet::new();
    requests
        .iter()
        .filter(|r| targets.contains(r.id.as_str()))
        .map(|r| (r.id.clone(), make_ref(&r.name, &mut taken)))
        .collect()
}

fn collection_entry(node: &CollectionTreeItem, refs: &HashMap<String, String>) -> CollectionEntry {
    CollectionEntry {
        name: node.name.clone(),
        description: non_empty(node.description.clone()),
        requests: node.requests.iter().map(|r| request_entry(r, refs)).collect(),
        collections: node.children.iter().map(|c| collection_entry(c, refs)).collect(),
    }
}

fn request_entry(req: &RequestItem, refs: &HashMap<String, String>) -> RequestEntry {
    let steps = |ids: &[String]| ids.iter().filter_map(|id| refs.get(id).cloned()).collect();
    RequestEntry {
        reference: refs.get(&req.id).cloned(),
        name: req.name.clone(),
        method: req.method.to_uppercase(),
        url: req.url.clone(),
        description: non_empty(req.description.clone()),
        params: req.params.iter().filter(|p| !blank(&p.key, &p.value)).map(param_entry).collect(),
        headers: req.headers.iter().filter(|h| !blank(&h.key, &h.value)).map(header_entry).collect(),
        body: body(req),
        extract: req
            .extract_rules
            .iter()
            .filter_map(|rule| {
                Some(ExtractEntry {
                    variable: rule.target_variable.clone(),
                    from: ExtractSource::from_stored(&rule.r#type)?,
                    expression: rule.expression.clone(),
                    enabled: rule.enabled,
                })
            })
            .collect(),
        before: steps(&req.pre_requests),
        after: steps(&req.post_requests),
    }
}

/// Empty rows the editor keeps for typing into aren't worth exporting.
fn blank(key: &str, value: &str) -> bool {
    key.trim().is_empty() && value.trim().is_empty()
}

fn param_entry(p: &ParamItem) -> KeyValue {
    KeyValue { name: p.key.clone(), value: p.value.clone(), enabled: p.enabled }
}

fn header_entry(h: &HeaderItem) -> KeyValue {
    KeyValue { name: h.key.clone(), value: h.value.clone(), enabled: h.enabled }
}

fn body(req: &RequestItem) -> Option<Body> {
    let text = |field: &Option<String>| field.clone().unwrap_or_default();
    match req.body_type.as_str() {
        "json" => Some(Body::Json { content: req.body_json.clone().or_else(|| req.body_raw.clone()).unwrap_or_default() }),
        "raw" | "binary" => Some(Body::Raw { content: text(&req.body_raw) }),
        "urlencoded" | "x-www-form-urlencoded" => {
            let rows: Vec<UrlEncodedParamSpec> =
                serde_json::from_str(req.body_urlencoded.as_deref().unwrap_or("[]")).unwrap_or_default();
            Some(Body::Urlencoded {
                fields: rows
                    .into_iter()
                    .filter(|r| !blank(&r.key, &r.value))
                    .map(|r| KeyValue { name: r.key, value: r.value, enabled: r.enabled })
                    .collect(),
            })
        }
        "form" | "form-data" | "multipart" => {
            let content = req.body_form_data.as_deref().unwrap_or("[]");
            let fields: Vec<MultipartFieldSpec> = serde_json::from_str::<MultipartBodySpec>(content)
                .map(|spec| spec.__form_data)
                .or_else(|_| serde_json::from_str(content))
                .unwrap_or_default();
            Some(Body::Form {
                fields: fields
                    .into_iter()
                    .filter(|f| !blank(&f.key, &f.value))
                    .map(|f| FormField {
                        name: f.key,
                        value: f.value,
                        kind: FormFieldType::from_stored(&f.r#type),
                        file_name: f.file_name,
                        content_type: f.content_type,
                        enabled: f.enabled,
                    })
                    .collect(),
            })
        }
        _ => None,
    }
}

fn non_empty(text: Option<String>) -> Option<String> {
    text.filter(|t| !t.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::import::import_workspace_json_db;

    const EXAMPLE: &str = include_str!("../../../../examples/online-store.mitm.json");

    fn temp_db() -> (crate::db::Db, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("mitm-export-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = crate::db::Db::new(dir.join("test.db"));
        crate::db::init_database(&db, None).unwrap();
        (db, dir)
    }

    #[test]
    fn importing_the_example_and_exporting_it_gives_the_same_collections() {
        let (db, dir) = temp_db();
        let summary = import_workspace_json_db(&db, EXAMPLE, None, None, None).unwrap();
        let exported = WorkspaceFile::parse(&export_workspace_json_db(&db, &summary.workspace_id, ExportOptions { include_secrets: true, include_auto: true }, None).unwrap()).unwrap();
        let original = WorkspaceFile::parse(EXAMPLE).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(exported.workspace, original.workspace);
        assert_eq!(exported.collections, original.collections);
        assert_eq!(exported.environments.len(), original.environments.len());
        for (got, want) in exported.environments.iter().zip(&original.environments) {
            assert_eq!((&got.name, got.active), (&want.name, want.active));
            for (g, w) in got.variables.iter().zip(&want.variables) {
                let (g, w) = (g.to_variable(), w.to_variable());
                assert_eq!((g.current_value(), g.variants.len(), g.r#type.as_str()), (w.current_value(), w.variants.len(), w.r#type.as_str()));
            }
        }
    }

    #[test]
    fn exporting_twice_gives_the_same_file_and_secrets_stay_blank() {
        let (db, dir) = temp_db();
        let summary = import_workspace_json_db(&db, EXAMPLE, None, None, None).unwrap();
        let first = export_workspace_json_db(&db, &summary.workspace_id, ExportOptions::default(), None).unwrap();
        let second = export_workspace_json_db(&db, &summary.workspace_id, ExportOptions::default(), None).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(first, second);
        assert!(!first.contains("local-only-password"));
        assert!(!first.contains("\"id\""));
    }

    fn workspace_with_credentials(db: &crate::db::Db) -> String {
        use crate::credentials::{create_credential, save_credential, Field, FieldKind};
        use crate::workspace::{save_workspace_environment_db, Environment, EnvironmentVariable, VariableVariant};
        let ws = crate::workspace::create_workspace_db(db, "W".into(), None).unwrap().id;
        let mut cred = create_credential(db, &crate::credentials::tests_support::vault(db, &ws, "DG"), "dgAdmin").unwrap();
        let field = |key: &str, kind, extracted, value: &str| Field { key: key.into(), kind, extracted, value: value.into() };
        cred.fields = vec![
            field("password", FieldKind::Secret, false, "hunter2"),
            field("accessToken", FieldKind::Secret, true, "eyJtoken"),
        ];
        cred.notes = "rotate monthly".into();
        save_credential(db, &cred).unwrap();
        let link = |value: &str| VariableVariant { name: "dev".into(), value: value.into() };
        save_workspace_environment_db(db, Environment {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: ws.clone(),
            name: "dev".into(),
            is_active: true,
            variables: vec![
                EnvironmentVariable {
                    key: "adminPassword".into(),
                    value: String::new(),
                    enabled: true,
                    r#type: "secret".into(),
                    active_index: 0,
                    variants: vec![link("{{$cred:DG/dgAdmin.password}}")],
                },
                EnvironmentVariable {
                    key: "accessToken".into(),
                    value: String::new(),
                    enabled: true,
                    r#type: "default".into(),
                    active_index: 0,
                    variants: vec![VariableVariant { name: "(auto)".into(), value: "{{$cred:DG/dgAdmin.accessToken}}".into() }],
                },
            ],
            created_at_ms: 0,
            updated_at_ms: 0,
        })
        .unwrap();
        ws
    }

    #[test]
    fn credentials_export_without_values_unless_sealed() {
        let (db, dir) = temp_db();
        let ws = workspace_with_credentials(&db);

        let plain = export_workspace_json_db(&db, &ws, ExportOptions::default(), None).unwrap();
        assert!(!plain.contains("hunter2") && !plain.contains("eyJtoken") && !plain.contains("rotate"));
        let file = WorkspaceFile::parse(&plain).unwrap();
        assert_eq!(file.vaults[0].name, "DG");
        assert_eq!(file.vaults[0].credentials[0].name, "dgAdmin");
        assert_eq!(file.vaults[0].credentials[0].fields.len(), 2);
        assert!(file.credentials.is_empty());
        assert!(file.credentials_sealed.is_none());
        assert!(plain.contains("{{$cred:DG/dgAdmin.password}}"), "links survive even on secret variables");
        assert!(plain.contains("{{$cred:DG/dgAdmin.accessToken}}"), "an (auto) link survives");

        let sealed = export_workspace_json_db(&db, &ws, ExportOptions::default(), Some("pw")).unwrap();
        assert!(!sealed.contains("hunter2"));
        let entries = crate::credentials::open_sealed(&WorkspaceFile::parse(&sealed).unwrap().credentials_sealed.unwrap(), "pw").unwrap();
        assert_eq!(entries[0].values.get("password").map(String::as_str), Some("hunter2"));
        assert!(!entries[0].values.contains_key("accessToken"), "extracted values need include_auto");

        let with_auto = ExportOptions { include_auto: true, ..Default::default() };
        let sealed = export_workspace_json_db(&db, &ws, with_auto, Some("pw")).unwrap();
        let entries = crate::credentials::open_sealed(&WorkspaceFile::parse(&sealed).unwrap().credentials_sealed.unwrap(), "pw").unwrap();
        assert_eq!(entries[0].values.get("accessToken").map(String::as_str), Some("eyJtoken"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sealed_credentials_import_with_the_passphrase_and_links_resolve() {
        let (db, dir) = temp_db();
        let ws = workspace_with_credentials(&db);
        let json = export_workspace_json_db(&db, &ws, ExportOptions::default(), Some("pw")).unwrap();

        let (elsewhere, other_dir) = temp_db();
        assert!(import_workspace_json_db(&elsewhere, &json, None, Some("Wrong"), Some("nope")).is_err());
        let preview = crate::workspace::import::preview_workspace_import_db(&elsewhere, &json).unwrap();
        assert!(preview.credentials_sealed);
        assert_eq!(preview.summary.credentials_imported, 1);

        let summary = import_workspace_json_db(&elsewhere, &json, None, Some("Copy"), Some("pw")).unwrap();
        assert_eq!(summary.credentials_imported, 1);
        let copy = crate::credentials::find_credential(&elsewhere, &summary.workspace_id, "DG", "dgAdmin").unwrap().unwrap();
        assert_eq!(copy.field("password").unwrap().value, "hunter2");
        assert_eq!(copy.notes, "rotate monthly");
        let resolved = crate::workspace::interpolate_variables(&elsewhere, &summary.workspace_id, "{{adminPassword}}");
        assert_eq!(resolved, "hunter2");

        let skipped = import_workspace_json_db(&elsewhere, &json, None, Some("Names only"), None).unwrap();
        assert_eq!(skipped.credentials_imported, 0, "the vault already has every entry and field");

        let (fresh, fresh_dir) = temp_db();
        let names = import_workspace_json_db(&fresh, &json, None, Some("Names only"), None).unwrap();
        let names_only = crate::credentials::find_credential(&fresh, &names.workspace_id, "DG", "dgAdmin").unwrap().unwrap();
        assert_eq!(names_only.fields.len(), 2);
        assert!(names_only.fields.iter().all(|f| f.value.is_empty()));
        for dir in [dir, other_dir, fresh_dir] {
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn importing_where_the_vault_exists_links_it_instead_of_copying() {
        let (db, dir) = temp_db();
        let ws = workspace_with_credentials(&db);
        let json = export_workspace_json_db(&db, &ws, ExportOptions::default(), None).unwrap();
        let summary = import_workspace_json_db(&db, &json, None, Some("Copy"), None).unwrap();

        let vaults = crate::credentials::list_vaults(&db).unwrap();
        assert_eq!(vaults.len(), 1);
        assert!(vaults[0].is_linked(&ws) && vaults[0].is_linked(&summary.workspace_id));
        let resolved = crate::workspace::interpolate_variables(&db, &summary.workspace_id, "{{adminPassword}}");
        assert_eq!(resolved, "hunter2");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn files_from_before_vaults_import_into_a_vault_named_after_the_workspace() {
        let (db, dir) = temp_db();
        let entries = vec![crate::credentials::SealedEntry {
            vault: String::new(),
            name: "dgAdmin".into(),
            values: [("password".to_string(), "hunter2".to_string())].into(),
            notes: String::new(),
        }];
        let sealed = serde_json::to_value(crate::credentials::seal("pw", &entries).unwrap()).unwrap();
        let json = serde_json::json!({
            "format": super::super::file::FORMAT,
            "version": 1,
            "workspace": { "name": "DG Assessment" },
            "environments": [{ "name": "dev", "active": true, "variables": [
                { "key": "adminPassword", "value": "{{$cred:dgAdmin.password}}" }
            ]}],
            "credentials": [{ "name": "dgAdmin", "fields": [{ "key": "password", "kind": "secret" }] }],
            "credentialsSealed": sealed,
        })
        .to_string();

        let summary = import_workspace_json_db(&db, &json, None, None, Some("pw")).unwrap();
        let entry = crate::credentials::find_credential(&db, &summary.workspace_id, "DG-Assessment", "dgAdmin").unwrap().unwrap();
        assert_eq!(entry.field("password").unwrap().value, "hunter2");
        let resolved = crate::workspace::interpolate_variables(&db, &summary.workspace_id, "{{adminPassword}}");
        assert_eq!(resolved, "hunter2");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
