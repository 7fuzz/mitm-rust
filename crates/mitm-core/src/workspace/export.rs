use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use rusqlite::{params, Connection};

use crate::collections::{get_collections_db, CollectionTreeItem, RequestItem};
use crate::encoding::{MultipartBodySpec, MultipartFieldSpec, UrlEncodedParamSpec};
use crate::repeater::{HeaderItem, ParamItem};
use crate::workspace::get_workspace_environments_db;
use super::file::{
    make_ref, Body, ExportOptions, CollectionEntry, EnvironmentEntry, ExtractEntry, ExtractSource, FormField, FormFieldType, KeyValue,
    RequestEntry, VariableEntry, WorkspaceFile, WorkspaceInfo,
};

/// The workspace as a `mitm-workspace` file.
pub fn export_workspace_json_db(db_path: &PathBuf, workspace_id: &str, options: ExportOptions) -> Result<String, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let (name, description): (String, Option<String>) = conn
        .query_row("SELECT name, description FROM workspaces WHERE id = ?", params![workspace_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|e| format!("Workspace with ID '{}' not found: {}", workspace_id, e))?;

    let mut file = WorkspaceFile::new(WorkspaceInfo { name, description: non_empty(description) });
    file.environments = get_workspace_environments_db(db_path, workspace_id)?
        .iter()
        .map(|env| EnvironmentEntry {
            name: env.name.clone(),
            active: env.is_active,
            variables: env.variables.iter().map(|v| VariableEntry::from_variable(v, options)).collect(),
        })
        .collect();

    let tree = get_collections_db(db_path, workspace_id)?;
    let refs = step_refs(&tree);
    file.collections = tree.iter().map(|node| collection_entry(node, &refs)).collect();

    serde_json::to_string_pretty(&file).map_err(|e| format!("Failed to serialize workspace: {}", e))
}

pub fn export_workspace_file_db(
    db_path: &PathBuf,
    workspace_id: &str,
    destination_path: &str,
    options: ExportOptions,
) -> Result<(), String> {
    let json = export_workspace_json_db(db_path, workspace_id, options)?;
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

    fn temp_db() -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("mitm-export-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("test.db");
        crate::db::init_database(&db, None).unwrap();
        (db, dir)
    }

    #[test]
    fn importing_the_example_and_exporting_it_gives_the_same_collections() {
        let (db, dir) = temp_db();
        let summary = import_workspace_json_db(&db, EXAMPLE, None, None).unwrap();
        let exported = WorkspaceFile::parse(&export_workspace_json_db(&db, &summary.workspace_id, ExportOptions { include_secrets: true, include_auto: true }).unwrap()).unwrap();
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
        let summary = import_workspace_json_db(&db, EXAMPLE, None, None).unwrap();
        let first = export_workspace_json_db(&db, &summary.workspace_id, ExportOptions::default()).unwrap();
        let second = export_workspace_json_db(&db, &summary.workspace_id, ExportOptions::default()).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(first, second);
        assert!(!first.contains("local-only-password"));
        assert!(!first.contains("\"id\""));
    }
}
