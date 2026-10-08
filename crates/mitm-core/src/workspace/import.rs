use std::path::PathBuf;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::encoding::{MultipartBodySpec, MultipartFieldSpec, UrlEncodedParamSpec};
use crate::repeater::{ExtractRuleItem, HeaderItem, ParamItem};
use super::file::{self, Body, CollectionEntry, KeyValue, RequestEntry, WorkspaceFile};

fn detect_and_assign_body(
    mode_hint: Option<&str>,
    raw_payload: Option<&str>,
) -> (String, Option<String>, Option<String>, Option<String>, Option<String>) {
    // Returns: (body_type, body_json, body_raw, body_form_data, body_urlencoded)
    let mode = mode_hint.unwrap_or("").to_lowercase();
    let payload = raw_payload.unwrap_or("").trim();

    if payload.is_empty() && mode.is_empty() {
        return ("none".to_string(), None, None, None, None);
    }

    let b_raw = if payload.is_empty() { None } else { Some(payload.to_string()) };

    // Explicit mode handling
    if mode == "formdata" || mode == "form-data" || mode == "multipart" {
        return ("form-data".to_string(), None, b_raw.clone(), b_raw, None);
    }

    if mode == "urlencoded" || mode == "x-www-form-urlencoded" {
        return ("urlencoded".to_string(), None, b_raw.clone(), None, b_raw);
    }

    if mode == "json" {
        return ("json".to_string(), b_raw.clone(), b_raw, None, None);
    }

    if mode == "raw" {
        // If mode is raw but content is valid JSON, auto-assign both json AND raw!
        if !payload.is_empty() && serde_json::from_str::<serde_json::Value>(payload).is_ok() {
            return ("json".to_string(), b_raw.clone(), b_raw, None, None);
        }
        return ("raw".to_string(), None, b_raw, None, None);
    }

    // Auto-detection when mode is unspecified or "none" but payload exists
    if !payload.is_empty() {
        if serde_json::from_str::<serde_json::Value>(payload).is_ok() {
            return ("json".to_string(), b_raw.clone(), b_raw, None, None);
        }
        return ("raw".to_string(), None, b_raw, None, None);
    }

    ("none".to_string(), None, None, None, None)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub workspace_id: String,
    pub workspace_name: String,
    pub environments_imported: usize,
    pub collections_imported: usize,
    pub requests_imported: usize,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PostmanVariable {
    key: String,
    value: Option<String>,
    #[serde(default)]
    disabled: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PostmanHeader {
    key: String,
    value: String,
    #[serde(default)]
    disabled: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PostmanUrl {
    raw: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PostmanUrlType {
    String(String),
    Object(PostmanUrl),
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PostmanBody {
    mode: Option<String>,
    raw: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PostmanRequest {
    method: Option<String>,
    url: Option<PostmanUrlType>,
    header: Option<Vec<PostmanHeader>>,
    body: Option<PostmanBody>,
    description: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PostmanItem {
    name: String,
    request: Option<PostmanRequest>,
    item: Option<Vec<PostmanItem>>,
    description: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PostmanInfo {
    name: String,
    description: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PostmanCollection {
    info: Option<PostmanInfo>,
    item: Option<Vec<PostmanItem>>,
    variable: Option<Vec<PostmanVariable>>,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportFormat {
    Workspace,
    Postman,
    Unknown,
}

impl ImportFormat {
    pub fn label(self) -> &'static str {
        match self {
            ImportFormat::Workspace => "Workspace file",
            ImportFormat::Postman => "Postman v2.1",
            ImportFormat::Unknown => "Unknown format",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub format: ImportFormat,
    pub summary: ImportSummary,
}

pub fn detect_import_format(parsed: &serde_json::Value) -> ImportFormat {
    if parsed.get("format").and_then(|f| f.as_str()) == Some(file::FORMAT) {
        return ImportFormat::Workspace;
    }
    let schema = parsed.get("info").and_then(|i| i.get("schema")).and_then(|s| s.as_str()).unwrap_or("");
    if schema.contains("postman") || parsed.get("item").is_some_and(|i| i.is_array()) {
        return ImportFormat::Postman;
    }
    ImportFormat::Unknown
}

pub fn import_workspace_json_db(
    db_path: &PathBuf,
    json_content: &str,
    target_workspace_id: Option<&str>,
    custom_workspace_name: Option<&str>,
) -> Result<ImportSummary, String> {
    let mut conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let summary = import_into(&tx, json_content, target_workspace_id, custom_workspace_name)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(summary)
}

/// Runs the real import as a new workspace and rolls it back, so the counts match what an import would do.
pub fn preview_workspace_import_db(db_path: &PathBuf, json_content: &str) -> Result<ImportPreview, String> {
    let mut conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let summary = import_into(&tx, json_content, None, None)?;
    tx.rollback().map_err(|e| e.to_string())?;
    let parsed: serde_json::Value = serde_json::from_str(json_content).map_err(|e| e.to_string())?;
    Ok(ImportPreview { format: detect_import_format(&parsed), summary })
}

fn import_into(
    conn: &Connection,
    json_content: &str,
    target_workspace_id: Option<&str>,
    custom_workspace_name: Option<&str>,
) -> Result<ImportSummary, String> {
    let parsed: serde_json::Value = serde_json::from_str(json_content).map_err(|e| format!("Invalid JSON: {}", e))?;
    match detect_import_format(&parsed) {
        ImportFormat::Workspace => {
            import_workspace_file(conn, WorkspaceFile::parse(json_content)?, target_workspace_id, custom_workspace_name)
        }
        ImportFormat::Postman => {
            let collection: PostmanCollection =
                serde_json::from_str(json_content).map_err(|e| format!("Invalid Postman collection: {}", e))?;
            import_postman(conn, collection, target_workspace_id, custom_workspace_name)
        }
        ImportFormat::Unknown => Err(format!(
            "Unrecognized file. Import reads workspace files (\"format\": \"{}\") and Postman v2.1 collections.",
            file::FORMAT
        )),
    }
}

fn import_postman(
    conn: &Connection,
    collection: PostmanCollection,
    target_workspace_id: Option<&str>,
    custom_workspace_name: Option<&str>,
) -> Result<ImportSummary, String> {
    let ts = now_ms();
    let name = custom_workspace_name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .or_else(|| collection.info.as_ref().map(|i| i.name.clone()))
        .unwrap_or_else(|| "Imported Workspace".to_string());
    let description = collection.info.as_ref().and_then(|i| i.description.clone());
    let (workspace_id, workspace_name, created) = open_workspace(conn, target_workspace_id, name, description, ts)?;

    let variables: Vec<super::EnvironmentVariable> = collection
        .variable
        .unwrap_or_default()
        .into_iter()
        .map(|v| {
            file::VariableEntry {
                key: v.key,
                value: v.value.unwrap_or_default(),
                enabled: !v.disabled,
                secret: false,
                variants: Vec::new(),
                active_variant: None,
            }
            .to_variable()
        })
        .collect();
    let mut env_count = 0;
    if !variables.is_empty() {
        let env_id = Uuid::new_v4().to_string();
        let vars_json = serde_json::to_string(&variables).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO environments (id, workspace_id, name, is_active, variables_json, created_at_ms, updated_at_ms)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![env_id, workspace_id, format!("{} variables", workspace_name), created, vars_json, ts, ts],
        )
        .map_err(|e| e.to_string())?;
        if created {
            conn.execute("UPDATE workspaces SET active_environment_id = ? WHERE id = ?", params![env_id, workspace_id])
                .map_err(|e| e.to_string())?;
        }
        env_count = 1;
    }

    let mut col_count = 0;
    let mut req_count = 0;
    for (idx, item) in collection.item.unwrap_or_default().into_iter().enumerate() {
        process_postman_item(conn, &workspace_id, None, item, idx as i32, ts, &mut col_count, &mut req_count)?;
    }

    Ok(ImportSummary {
        workspace_id,
        workspace_name,
        environments_imported: env_count,
        collections_imported: col_count,
        requests_imported: req_count,
    })
}

/// Creates the target workspace, or checks that the one to import into exists. Returns (id, name, created).
fn open_workspace(
    conn: &Connection,
    target_workspace_id: Option<&str>,
    name: String,
    description: Option<String>,
    ts: i64,
) -> Result<(String, String, bool), String> {
    if let Some(target_id) = target_workspace_id {
        let name: String = conn
            .query_row("SELECT name FROM workspaces WHERE id = ?", params![target_id], |row| row.get(0))
            .map_err(|_| format!("Target workspace with ID '{}' does not exist", target_id))?;
        return Ok((target_id.to_string(), name, false));
    }
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO workspaces (id, name, description, active_environment_id, created_at_ms, updated_at_ms)
         VALUES (?, ?, ?, NULL, ?, ?)",
        params![id, name, description, ts, ts],
    )
    .map_err(|e| e.to_string())?;
    Ok((id, name, true))
}

fn import_workspace_file(
    conn: &Connection,
    file: WorkspaceFile,
    target_workspace_id: Option<&str>,
    custom_workspace_name: Option<&str>,
) -> Result<ImportSummary, String> {
    let ts = now_ms();
    let name = custom_workspace_name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map_or_else(|| file.workspace.name.clone(), str::to_string);
    let description = file.workspace.description.clone().filter(|d| !d.trim().is_empty());
    let (workspace_id, workspace_name, _) = open_workspace(conn, target_workspace_id, name, description, ts)?;

    let mut active_env = None;
    for env in &file.environments {
        let env_id = Uuid::new_v4().to_string();
        let variables: Vec<super::EnvironmentVariable> = env.variables.iter().map(|v| v.to_variable()).collect();
        let vars_json = serde_json::to_string(&variables).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO environments (id, workspace_id, name, is_active, variables_json, created_at_ms, updated_at_ms)
             VALUES (?, ?, ?, 0, ?, ?, ?)",
            params![env_id, workspace_id, env.name, vars_json, ts, ts],
        )
        .map_err(|e| e.to_string())?;
        if env.active && active_env.is_none() {
            active_env = Some(env_id);
        }
    }
    // Importing into a workspace keeps the environment it already uses.
    let has_active: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM environments WHERE workspace_id = ? AND is_active = 1)", params![workspace_id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if let (Some(env_id), false) = (&active_env, has_active) {
        conn.execute("UPDATE environments SET is_active = 1 WHERE id = ?", params![env_id]).map_err(|e| e.to_string())?;
        conn.execute("UPDATE workspaces SET active_environment_id = ? WHERE id = ?", params![env_id, workspace_id])
            .map_err(|e| e.to_string())?;
    }

    let mut ctx = FileImport { conn, workspace_id: &workspace_id, ts, collections: 0, requests: Vec::new() };
    for (index, collection) in file.collections.iter().enumerate() {
        ctx.collection(collection, None, index)?;
    }
    let ids: std::collections::HashMap<&str, &str> = ctx
        .requests
        .iter()
        .filter_map(|(id, entry)| Some((entry.reference.as_deref()?, id.as_str())))
        .collect();
    for (id, entry) in &ctx.requests {
        if entry.before.is_empty() && entry.after.is_empty() {
            continue;
        }
        let resolve = |refs: &[String]| -> Vec<String> { refs.iter().filter_map(|r| ids.get(r.as_str())).map(|id| id.to_string()).collect() };
        crate::request_steps::save_steps(conn, id, &resolve(&entry.before), &resolve(&entry.after)).map_err(|e| e.to_string())?;
    }

    Ok(ImportSummary {
        workspace_id: workspace_id.clone(),
        workspace_name,
        environments_imported: file.environments.len(),
        collections_imported: ctx.collections,
        requests_imported: ctx.requests.len(),
    })
}

struct FileImport<'a> {
    conn: &'a Connection,
    workspace_id: &'a str,
    ts: i64,
    collections: usize,
    /// (new id, entry), kept to link `before` and `after` once every request has an id.
    requests: Vec<(String, &'a RequestEntry)>,
}

impl<'a> FileImport<'a> {
    fn collection(&mut self, entry: &'a CollectionEntry, parent_id: Option<&str>, index: usize) -> Result<(), String> {
        let id = Uuid::new_v4().to_string();
        self.conn
            .execute(
                "INSERT INTO collections (id, workspace_id, parent_id, name, description, order_index, created_at_ms, updated_at_ms)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                params![id, self.workspace_id, parent_id, entry.name, entry.description, index as i32, self.ts, self.ts],
            )
            .map_err(|e| e.to_string())?;
        self.collections += 1;
        for (index, request) in entry.requests.iter().enumerate() {
            self.request(request, &id, index)?;
        }
        for (index, child) in entry.collections.iter().enumerate() {
            self.collection(child, Some(&id), index)?;
        }
        Ok(())
    }

    fn request(&mut self, entry: &'a RequestEntry, collection_id: &str, index: usize) -> Result<(), String> {
        let id = Uuid::new_v4().to_string();
        let headers: Vec<HeaderItem> = entry
            .headers
            .iter()
            .map(|h| HeaderItem { id: Uuid::new_v4().to_string(), key: h.name.clone(), value: h.value.clone(), enabled: h.enabled })
            .collect();
        let params: Vec<ParamItem> = entry
            .params
            .iter()
            .map(|p| ParamItem { id: Uuid::new_v4().to_string(), key: p.name.clone(), value: p.value.clone(), enabled: p.enabled })
            .collect();
        let extract: Vec<ExtractRuleItem> = entry
            .extract
            .iter()
            .map(|e| ExtractRuleItem {
                id: Uuid::new_v4().to_string(),
                r#type: e.from.stored().to_string(),
                expression: e.expression.clone(),
                target_variable: e.variable.clone(),
                enabled: e.enabled,
            })
            .collect();
        let body = StoredBody::from(entry.body.as_ref());
        self.conn
            .execute(
                "INSERT INTO requests (id, collection_id, name, method, url, headers_json, params_json, body_type, body_json, body_raw, body_form_data, body_urlencoded, extract_rules_json, description, order_index, created_at_ms, updated_at_ms)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    id,
                    collection_id,
                    entry.name,
                    entry.method.to_uppercase(),
                    entry.url,
                    to_json(&headers),
                    to_json(&params),
                    body.body_type,
                    body.json,
                    body.raw,
                    body.form_data,
                    body.urlencoded,
                    to_json(&extract),
                    entry.description,
                    index as i32,
                    self.ts,
                    self.ts
                ],
            )
            .map_err(|e| e.to_string())?;
        self.requests.push((id, entry));
        Ok(())
    }
}

/// A file body spread over the request table's body columns.
#[derive(Default)]
struct StoredBody {
    body_type: &'static str,
    json: Option<String>,
    raw: Option<String>,
    form_data: Option<String>,
    urlencoded: Option<String>,
}

impl From<Option<&Body>> for StoredBody {
    fn from(body: Option<&Body>) -> Self {
        let Some(body) = body else { return Self { body_type: "none", ..Self::default() } };
        match body {
            Body::Json { content } => Self { body_type: "json", json: Some(content.clone()), raw: Some(content.clone()), ..Self::default() },
            Body::Raw { content } => Self { body_type: "raw", raw: Some(content.clone()), ..Self::default() },
            Body::Urlencoded { fields } => {
                let rows: Vec<UrlEncodedParamSpec> = fields
                    .iter()
                    .map(|f: &KeyValue| UrlEncodedParamSpec {
                        id: Some(Uuid::new_v4().to_string()),
                        enabled: f.enabled,
                        key: f.name.clone(),
                        value: f.value.clone(),
                    })
                    .collect();
                Self { body_type: "urlencoded", urlencoded: serde_json::to_string(&rows).ok(), ..Self::default() }
            }
            Body::Form { fields } => {
                let spec = MultipartBodySpec {
                    __form_data: fields
                        .iter()
                        .map(|f| MultipartFieldSpec {
                            id: Some(Uuid::new_v4().to_string()),
                            enabled: f.enabled,
                            key: f.name.clone(),
                            value: f.value.clone(),
                            r#type: f.kind.stored().to_string(),
                            file_name: f.file_name.clone(),
                            content_type: f.content_type.clone(),
                        })
                        .collect(),
                };
                Self { body_type: "form-data", form_data: serde_json::to_string(&spec).ok(), ..Self::default() }
            }
        }
    }
}

fn to_json<T: Serialize>(rows: &[T]) -> String {
    serde_json::to_string(rows).unwrap_or_else(|_| "[]".to_string())
}

#[allow(clippy::too_many_arguments)]
fn process_postman_item(
    conn: &Connection,
    workspace_id: &str,
    parent_id: Option<String>,
    item: PostmanItem,
    order_idx: i32,
    ts: i64,
    col_count: &mut usize,
    req_count: &mut usize,
) -> Result<(), String> {
    if let Some(req_spec) = item.request {
        // It's a Request Item
        let req_id = Uuid::new_v4().to_string();
        let col_id = match parent_id {
            Some(pid) => pid,
            None => {
                // Create a root fallback collection if request is at root level
                let root_col_id = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO collections (id, workspace_id, parent_id, name, description, order_index, created_at_ms, updated_at_ms)
                     VALUES (?, ?, NULL, 'Root Requests', NULL, 0, ?, ?)",
                    params![root_col_id, workspace_id, ts, ts],
                ).map_err(|e| e.to_string())?;
                *col_count += 1;
                root_col_id
            }
        };

        let method = req_spec.method.unwrap_or_else(|| "GET".to_string()).to_uppercase();
        let raw_url = match req_spec.url {
            Some(PostmanUrlType::String(s)) => s,
            Some(PostmanUrlType::Object(obj)) => obj.raw.unwrap_or_else(|| "https://httpbin.org/get".to_string()),
            None => "https://httpbin.org/get".to_string(),
        };

        let mut headers = Vec::new();
        if let Some(h_list) = req_spec.header {
            for h in h_list {
                headers.push(serde_json::json!({
                    "id": Uuid::new_v4().to_string(),
                    "key": h.key,
                    "value": h.value,
                    "enabled": !h.disabled
                }));
            }
        }
        let headers_json = serde_json::to_string(&headers).unwrap_or_else(|_| "[]".to_string());

        let mode_hint = req_spec.body.as_ref().and_then(|b| b.mode.as_deref());
        let raw_payload = req_spec.body.as_ref().and_then(|b| b.raw.as_deref());
        let (body_type, b_json, b_raw, b_form, b_url) = detect_and_assign_body(mode_hint, raw_payload);

        conn.execute(
            "INSERT INTO requests (id, collection_id, name, method, url, headers_json, params_json, body_type, body_json, body_raw, body_form_data, body_urlencoded, extract_rules_json, description, order_index, created_at_ms, updated_at_ms)
             VALUES (?, ?, ?, ?, ?, ?, '[]', ?, ?, ?, ?, ?, '[]', ?, ?, ?, ?)",
            params![
                req_id,
                col_id,
                item.name,
                method,
                raw_url,
                headers_json,
                body_type,
                b_json,
                b_raw,
                b_form,
                b_url,
                item.description,
                order_idx,
                ts,
                ts
            ],
        ).map_err(|e| e.to_string())?;

        *req_count += 1;
    } else {
        // It's a Collection / Subfolder
        let col_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO collections (id, workspace_id, parent_id, name, description, order_index, created_at_ms, updated_at_ms)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![col_id, workspace_id, parent_id, item.name, item.description, order_idx, ts, ts],
        ).map_err(|e| e.to_string())?;
        *col_count += 1;

        if let Some(sub_items) = item.item {
            for (sub_idx, sub_item) in sub_items.into_iter().enumerate() {
                process_postman_item(
                    conn,
                    workspace_id,
                    Some(col_id.clone()),
                    sub_item,
                    sub_idx as i32,
                    ts,
                    col_count,
                    req_count,
                )?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const POSTMAN: &str = r#"{
        "info": {"name": "Sample API", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},
        "variable": [{"key": "host", "value": "api.example.com"}],
        "item": [
            {"name": "Users", "item": [
                {"name": "List users", "request": {"method": "GET", "url": "{{host}}/users"}},
                {"name": "Create user", "request": {"method": "POST", "url": "{{host}}/users"}}
            ]}
        ]
    }"#;

    fn temp_db() -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("mitm-import-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("test.db");
        crate::db::init_database(&db, None).unwrap();
        (db, dir)
    }

    fn workspace_count(db: &PathBuf) -> i64 {
        Connection::open(db).unwrap().query_row("SELECT COUNT(*) FROM workspaces", [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn preview_matches_import_and_writes_nothing() {
        let (db, dir) = temp_db();
        let before = workspace_count(&db);

        let preview = preview_workspace_import_db(&db, POSTMAN).unwrap();
        assert_eq!(preview.format, ImportFormat::Postman);
        assert_eq!(preview.summary.workspace_name, "Sample API");
        assert_eq!(workspace_count(&db), before);

        let summary = import_workspace_json_db(&db, POSTMAN, None, None).unwrap();
        assert_eq!(summary.environments_imported, preview.summary.environments_imported);
        assert_eq!(summary.collections_imported, preview.summary.collections_imported);
        assert_eq!(summary.requests_imported, preview.summary.requests_imported);
        assert_eq!(summary.requests_imported, 2);
        assert_eq!(workspace_count(&db), before + 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    const EXAMPLE: &str = include_str!("../../../../examples/online-store.mitm.json");

    #[test]
    fn workspace_file_imports_environments_requests_and_steps() {
        let (db, dir) = temp_db();
        let summary = import_workspace_json_db(&db, EXAMPLE, None, None).unwrap();
        assert_eq!(summary.workspace_name, "Online Store");
        assert_eq!(
            (summary.environments_imported, summary.collections_imported, summary.requests_imported),
            (2, 4, 8)
        );

        let envs = super::super::get_workspace_environments_db(&db, &summary.workspace_id).unwrap();
        let local = envs.iter().find(|e| e.name == "Local").unwrap();
        assert!(local.is_active && !envs.iter().find(|e| e.name == "Staging").unwrap().is_active);
        let email = local.variables.iter().find(|v| v.key == "customerEmail").unwrap();
        let names: Vec<&str> = email.variants.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["(auto)", "default", "new customer", "returning"]);
        assert_eq!(email.current_value(), "returning.shopper@example.com");
        assert_eq!(local.variables.iter().find(|v| v.key == "customerPassword").unwrap().r#type, "secret");

        let tree = crate::collections::get_collections_db(&db, &summary.workspace_id).unwrap();
        let requests: Vec<_> = {
            fn walk<'a>(nodes: &'a [crate::collections::CollectionTreeItem], out: &mut Vec<&'a crate::collections::RequestItem>) {
                for node in nodes {
                    out.extend(&node.requests);
                    walk(&node.children, out);
                }
            }
            let mut out = Vec::new();
            walk(&tree, &mut out);
            out
        };
        let by_name = |name: &str| requests.iter().find(|r| r.name == name).unwrap();
        let login = by_name("Log in");
        assert_eq!(login.body_type, "urlencoded");
        assert!(login.body_urlencoded.as_deref().unwrap().contains("\"enabled\":false"));
        assert_eq!(login.extract_rules[0].target_variable, "accessToken");
        let order = by_name("Place order");
        assert_eq!(order.pre_requests, [login.id.clone(), by_name("Add to cart").id.clone()]);
        assert_eq!(order.post_requests, [by_name("Log out").id.clone()]);
        assert_eq!(by_name("Upload product image").body_type, "form-data");
        assert!(!by_name("List products").params[2].enabled);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn importing_into_a_workspace_keeps_its_active_environment() {
        let (db, dir) = temp_db();
        let first = import_workspace_json_db(&db, EXAMPLE, None, None).unwrap();
        let envs = || super::super::get_workspace_environments_db(&db, &first.workspace_id).unwrap();
        let staging = envs().into_iter().find(|e| e.name == "Staging").unwrap();
        super::super::set_active_environment_db(&db, &first.workspace_id, Some(&staging.id)).unwrap();

        import_workspace_json_db(&db, EXAMPLE, Some(&first.workspace_id), None).unwrap();
        let active: Vec<String> = envs().into_iter().filter(|e| e.is_active).map(|e| e.id).collect();
        assert_eq!(active, [staging.id]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_rejected_workspace_file_writes_nothing() {
        let (db, dir) = temp_db();
        let before = workspace_count(&db);
        let broken = EXAMPLE.replace("\"before\": [\"log-in\"]", "\"before\": [\"sign-in\"]");
        assert!(import_workspace_json_db(&db, &broken, None, None).unwrap_err().contains("sign-in"));
        assert_eq!(workspace_count(&db), before);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detects_workspace_files_and_rejects_the_old_format() {
        let detect = |json: &str| detect_import_format(&serde_json::from_str(json).unwrap());
        assert_eq!(detect(r#"{"format": "mitm-workspace", "version": 1}"#), ImportFormat::Workspace);
        assert_eq!(detect(r#"{"test_cases": [], "all_environments": []}"#), ImportFormat::Unknown);

        let (db, dir) = temp_db();
        let err = import_workspace_json_db(&db, r#"{"name": "Old", "test_cases": []}"#, None, None).unwrap_err();
        assert!(err.contains("Unrecognized file"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
