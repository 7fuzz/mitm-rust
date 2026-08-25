use std::path::PathBuf;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub workspace_id: String,
    pub workspace_name: String,
    pub environments_imported: usize,
    pub collections_imported: usize,
    pub requests_imported: usize,
}

#[derive(Debug, Deserialize)]
struct PostmanVariable {
    key: String,
    value: Option<String>,
    #[serde(default)]
    disabled: bool,
}

#[derive(Debug, Deserialize)]
struct PostmanHeader {
    key: String,
    value: String,
    #[serde(default)]
    disabled: bool,
}

#[derive(Debug, Deserialize)]
struct PostmanUrl {
    raw: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PostmanUrlType {
    String(String),
    Object(PostmanUrl),
}

#[derive(Debug, Deserialize)]
struct PostmanBody {
    mode: Option<String>,
    raw: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PostmanRequest {
    method: Option<String>,
    url: Option<PostmanUrlType>,
    header: Option<Vec<PostmanHeader>>,
    body: Option<PostmanBody>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PostmanItem {
    name: String,
    request: Option<PostmanRequest>,
    item: Option<Vec<PostmanItem>>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PostmanInfo {
    name: String,
    description: Option<String>,
}

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

pub fn import_workspace_json_db(
    db_path: &PathBuf,
    json_content: &str,
) -> Result<ImportSummary, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();

    // 1. Try parsing as Postman v2.1 Collection or generic project JSON
    let parsed: serde_json::Value = serde_json::from_str(json_content).map_err(|e| format!("Invalid JSON format: {}", e))?;

    let workspace_id = Uuid::new_v4().to_string();
    let workspace_name = parsed.get("info")
        .and_then(|i| i.get("name"))
        .and_then(|n| n.as_str())
        .or_else(|| parsed.get("name").and_then(|n| n.as_str()))
        .unwrap_or("Imported Workspace")
        .to_string();

    let workspace_desc = parsed.get("info")
        .and_then(|i| i.get("description"))
        .and_then(|d| d.as_str())
        .or_else(|| parsed.get("description").and_then(|d| d.as_str()))
        .map(|s| s.to_string());

    // Insert Workspace
    conn.execute(
        "INSERT INTO workspaces (id, name, description, active_environment_id, created_at_ms, updated_at_ms)
         VALUES (?, ?, ?, NULL, ?, ?)",
        params![workspace_id, workspace_name, workspace_desc, ts, ts],
    ).map_err(|e| e.to_string())?;

    let mut env_count = 0;
    let mut col_count = 0;
    let mut req_count = 0;

    // 2. Import Environment Variables if present
    let mut env_vars = Vec::new();
    if let Some(vars) = parsed.get("variable").and_then(|v| v.as_array()) {
        for v in vars {
            if let Some(key) = v.get("key").and_then(|k| k.as_str()) {
                let val = v.get("value").and_then(|val| val.as_str()).unwrap_or("");
                let disabled = v.get("disabled").and_then(|d| d.as_bool()).unwrap_or(false);
                env_vars.push(serde_json::json!({
                    "key": key,
                    "value": val,
                    "enabled": !disabled,
                    "type": "default"
                }));
            }
        }
    }

    if !env_vars.is_empty() {
        let env_id = Uuid::new_v4().to_string();
        let vars_json = serde_json::to_string(&env_vars).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT INTO environments (id, workspace_id, name, is_active, variables_json, created_at_ms, updated_at_ms)
             VALUES (?, ?, 'Default Environment', 1, ?, ?, ?)",
            params![env_id, workspace_id, vars_json, ts, ts],
        ).map_err(|e| e.to_string())?;

        let _ = conn.execute(
            "UPDATE workspaces SET active_environment_id = ? WHERE id = ?",
            params![env_id, workspace_id],
        );
        env_count = 1;
    }

    // 3. Import Items (Folders & Requests)
    if let Ok(collection) = serde_json::from_str::<PostmanCollection>(json_content) {
        if let Some(items) = collection.item {
            for (idx, item) in items.into_iter().enumerate() {
                process_postman_item(
                    &conn,
                    &workspace_id,
                    None,
                    item,
                    idx as i32,
                    ts,
                    &mut col_count,
                    &mut req_count,
                )?;
            }
        }
    }

    Ok(ImportSummary {
        workspace_id,
        workspace_name,
        environments_imported: env_count,
        collections_imported: col_count,
        requests_imported: req_count,
    })
}

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

        let body_type = match req_spec.body.as_ref().and_then(|b| b.mode.as_deref()) {
            Some("raw") => "json",
            Some("formdata") => "form-data",
            _ => "none",
        };
        let body_content = req_spec.body.and_then(|b| b.raw);

        conn.execute(
            "INSERT INTO requests (id, collection_id, name, method, url, headers_json, params_json, body_type, body_content, extract_rules_json, description, order_index, created_at_ms, updated_at_ms)
             VALUES (?, ?, ?, ?, ?, ?, '[]', ?, ?, '[]', ?, ?, ?, ?)",
            params![
                req_id,
                col_id,
                item.name,
                method,
                raw_url,
                headers_json,
                body_type,
                body_content,
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
