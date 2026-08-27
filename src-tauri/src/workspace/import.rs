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

pub fn import_workspace_json_db(
    db_path: &PathBuf,
    json_content: &str,
    target_workspace_id: Option<&str>,
    custom_workspace_name: Option<&str>,
) -> Result<ImportSummary, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Self-healing schema check for legacy databases missing workspace_id columns
    let _ = conn.execute("ALTER TABLE environments ADD COLUMN workspace_id TEXT NOT NULL DEFAULT ''", []);
    let _ = conn.execute("ALTER TABLE environments ADD COLUMN variables_json TEXT NOT NULL DEFAULT '[]'", []);
    let _ = conn.execute("ALTER TABLE collections ADD COLUMN workspace_id TEXT NOT NULL DEFAULT ''", []);

    let ts = now_ms();

    // 1. Parse JSON content
    let parsed: serde_json::Value = serde_json::from_str(json_content).map_err(|e| format!("Invalid JSON format: {}", e))?;

    let (workspace_id, workspace_name) = match target_workspace_id {
        Some(target_id) => {
            // Import into existing workspace
            let name: String = conn.query_row(
                "SELECT name FROM workspaces WHERE id = ?",
                params![target_id],
                |row| row.get(0),
            ).map_err(|_| format!("Target workspace with ID '{}' does not exist", target_id))?;

            (target_id.to_string(), name)
        }
        None => {
            // Create a new workspace
            let ws_id = Uuid::new_v4().to_string();

            let parsed_name = parsed.get("info")
                .and_then(|i| i.get("name"))
                .and_then(|n| n.as_str())
                .or_else(|| parsed.get("name").and_then(|n| n.as_str()));

            let ws_name = match custom_workspace_name {
                Some(c_name) if !c_name.trim().is_empty() => c_name.trim().to_string(),
                _ => parsed_name.unwrap_or("Imported Workspace").to_string(),
            };

            let ws_desc = parsed.get("info")
                .and_then(|i| i.get("description"))
                .and_then(|d| d.as_str())
                .or_else(|| parsed.get("description").and_then(|d| d.as_str()))
                .map(|s| s.to_string());

            // Insert Workspace
            conn.execute(
                "INSERT INTO workspaces (id, name, description, active_environment_id, created_at_ms, updated_at_ms)
                 VALUES (?, ?, ?, NULL, ?, ?)",
                params![ws_id, ws_name, ws_desc, ts, ts],
            ).map_err(|e| e.to_string())?;

            (ws_id, ws_name)
        }
    };

    let mut env_count = 0;
    let mut col_count = 0;
    let mut req_count = 0;

    // 2. Import Environment Variables
    // Check for native project format: all_environments & all_variables
    if let Some(all_envs) = parsed.get("all_environments").and_then(|e| e.as_array()) {
        let all_vars = parsed.get("all_variables").and_then(|v| v.as_array());

        for env_obj in all_envs {
            let env_spec_id = env_obj.get("id").and_then(|i| i.as_str()).unwrap_or("");
            let env_spec_name = env_obj.get("name").and_then(|n| n.as_str()).unwrap_or("Imported Environment");

            let mut env_vars = Vec::new();
            if let Some(vars_array) = all_vars {
                for v_item in vars_array {
                    let v_env_id = v_item.get("environmentId").and_then(|e| e.as_str()).unwrap_or("");
                    if v_env_id == env_spec_id || env_spec_id.is_empty() {
                        if let Some(key) = v_item.get("name").and_then(|k| k.as_str()) {
                            let active_index = v_item.get("activeIndex").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                            let mut variants = Vec::new();

                            if let Some(vals_array) = v_item.get("values").and_then(|v| v.as_array()) {
                                for val_obj in vals_array {
                                    let var_name = val_obj.get("name").and_then(|n| n.as_str()).unwrap_or("Default").to_string();
                                    let var_val = val_obj.get("value")
                                        .map(|v| if v.is_string() { v.as_str().unwrap_or("").to_string() } else { v.to_string() })
                                        .unwrap_or_default();
                                    variants.push(serde_json::json!({
                                        "name": var_name,
                                        "value": var_val
                                    }));
                                }
                            }

                            let primary_val = if !variants.is_empty() {
                                let sel_idx = if active_index < variants.len() { active_index } else { 0 };
                                variants[sel_idx].get("value").and_then(|v| v.as_str()).unwrap_or("").to_string()
                            } else {
                                v_item.get("value")
                                    .map(|v| if v.is_string() { v.as_str().unwrap_or("").to_string() } else { v.to_string() })
                                    .unwrap_or_default()
                            };

                            env_vars.push(serde_json::json!({
                                "key": key,
                                "value": primary_val,
                                "enabled": true,
                                "type": "default",
                                "activeIndex": active_index,
                                "variants": variants
                            }));
                        }
                    }
                }
            }

            let env_id = Uuid::new_v4().to_string();
            let vars_json = serde_json::to_string(&env_vars).unwrap_or_else(|_| "[]".to_string());
            conn.execute(
                "INSERT INTO environments (id, workspace_id, name, is_active, variables_json, created_at_ms, updated_at_ms)
                 VALUES (?, ?, ?, 1, ?, ?, ?)",
                params![env_id, workspace_id, env_spec_name, vars_json, ts, ts],
            ).map_err(|e| e.to_string())?;

            let _ = conn.execute(
                "UPDATE workspaces SET active_environment_id = ? WHERE id = ?",
                params![env_id, workspace_id],
            );
            env_count += 1;
        }
    } else {
        // Fallback Postman variable array
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
            let env_name = format!("Imported Env ({})", workspace_name);
            conn.execute(
                "INSERT INTO environments (id, workspace_id, name, is_active, variables_json, created_at_ms, updated_at_ms)
                 VALUES (?, ?, ?, 1, ?, ?, ?)",
                params![env_id, workspace_id, env_name, vars_json, ts, ts],
            ).map_err(|e| e.to_string())?;

            let _ = conn.execute(
                "UPDATE workspaces SET active_environment_id = ? WHERE id = ?",
                params![env_id, workspace_id],
            );
            env_count = 1;
        }
    }

    // 3. Import Items (Folders & Requests)
    // Check for native project test_cases
    if let Some(test_cases) = parsed.get("test_cases").and_then(|t| t.as_array()) {
        for (idx, tc) in test_cases.iter().enumerate() {
            process_custom_folder(
                &conn,
                &workspace_id,
                None,
                tc,
                idx as i32,
                ts,
                &mut col_count,
                &mut req_count,
            )?;
        }
    } else if let Ok(collection) = serde_json::from_str::<PostmanCollection>(json_content) {
        // Fallback Postman v2.1 collection format
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

fn process_custom_folder(
    conn: &Connection,
    workspace_id: &str,
    parent_id: Option<String>,
    folder_val: &serde_json::Value,
    order_idx: i32,
    ts: i64,
    col_count: &mut usize,
    req_count: &mut usize,
) -> Result<(), String> {
    let name = folder_val.get("name").and_then(|n| n.as_str()).unwrap_or("Untitled Group");
    let desc = folder_val.get("description").and_then(|d| d.as_str()).map(|s| s.to_string());

    let col_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO collections (id, workspace_id, parent_id, name, description, order_index, created_at_ms, updated_at_ms)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![col_id, workspace_id, parent_id, name, desc, order_idx, ts, ts],
    ).map_err(|e| e.to_string())?;
    *col_count += 1;

    // Process target array (requests) inside folder
    if let Some(targets) = folder_val.get("target").and_then(|t| t.as_array()) {
        for (idx, target) in targets.iter().enumerate() {
            process_custom_target(conn, &col_id, target, idx as i32, ts, req_count)?;
        }
    }

    // Process nested sub-folders
    if let Some(sub_folders) = folder_val.get("folders").and_then(|f| f.as_array()) {
        for (idx, sub_f) in sub_folders.iter().enumerate() {
            process_custom_folder(
                conn,
                workspace_id,
                Some(col_id.clone()),
                sub_f,
                idx as i32,
                ts,
                col_count,
                req_count,
            )?;
        }
    }

    Ok(())
}

fn process_custom_target(
    conn: &Connection,
    col_id: &str,
    target_val: &serde_json::Value,
    order_idx: i32,
    ts: i64,
    req_count: &mut usize,
) -> Result<(), String> {
    let req_id = Uuid::new_v4().to_string();
    let name = target_val.get("name").and_then(|n| n.as_str()).unwrap_or("Untitled Request");
    let method = target_val.get("method").and_then(|m| m.as_str()).unwrap_or("GET").to_uppercase();

    let raw_url = target_val.get("endpoint")
        .and_then(|u| u.as_str())
        .or_else(|| target_val.get("url").and_then(|u| u.as_str()))
        .unwrap_or("https://httpbin.org/get");

    let desc = target_val.get("description").and_then(|d| d.as_str()).map(|s| s.to_string());

    // Parse headers (supports Object {"K":"V"} or Array [{"key":"K","value":"V"}])
    let mut headers = Vec::new();
    if let Some(h_obj) = target_val.get("header").and_then(|h| h.as_object()) {
        for (k, v) in h_obj {
            if let Some(val_str) = v.as_str() {
                headers.push(serde_json::json!({
                    "id": Uuid::new_v4().to_string(),
                    "key": k,
                    "value": val_str,
                    "enabled": true
                }));
            }
        }
    } else if let Some(h_arr) = target_val.get("header").and_then(|h| h.as_array()) {
        for h in h_arr {
            if let Some(k) = h.get("key").and_then(|k| k.as_str()) {
                let v = h.get("value").and_then(|v| v.as_str()).unwrap_or("");
                let disabled = h.get("disabled").and_then(|d| d.as_bool()).unwrap_or(false);
                headers.push(serde_json::json!({
                    "id": Uuid::new_v4().to_string(),
                    "key": k,
                    "value": v,
                    "enabled": !disabled
                }));
            }
        }
    }
    let headers_json = serde_json::to_string(&headers).unwrap_or_else(|_| "[]".to_string());

    // Body handling
    let body_mode = target_val.get("body_mode").and_then(|m| m.as_str()).unwrap_or("none");
    let body_type = match body_mode {
        "json" | "raw" => "json",
        "formdata" | "form-data" | "urlencoded" => "form-data",
        _ => "none",
    };

    let body_content = target_val.get("body")
        .and_then(|b| if b.is_string() { b.as_str().map(|s| s.to_string()) } else { Some(b.to_string()) })
        .or_else(|| target_val.get("body_json").and_then(|b| if b.is_string() { b.as_str().map(|s| s.to_string()) } else { Some(b.to_string()) }));

    // Extraction rules handling (supports dict {"var": "expr"} or array of objects [{"type":"...","targetVariable":"...","expression":"..."}])
    let mut extract_rules = Vec::new();
    let extract_field = target_val.get("extract")
        .or_else(|| target_val.get("extract_rules"))
        .or_else(|| target_val.get("extraction"))
        .or_else(|| target_val.get("extractions"));

    if let Some(ext_obj) = extract_field.and_then(|e| e.as_object()) {
        for (target_var, expr_val) in ext_obj {
            if let Some(expr_str) = expr_val.as_str() {
                extract_rules.push(serde_json::json!({
                    "id": Uuid::new_v4().to_string(),
                    "type": "json",
                    "targetVariable": target_var,
                    "expression": expr_str,
                    "enabled": true
                }));
            }
        }
    } else if let Some(ext_arr) = extract_field.and_then(|e| e.as_array()) {
        for rule in ext_arr {
            let target_var = rule.get("targetVariable")
                .or_else(|| rule.get("target_variable"))
                .or_else(|| rule.get("variableName"))
                .or_else(|| rule.get("varName"))
                .or_else(|| rule.get("var"))
                .or_else(|| rule.get("key"))
                .and_then(|v| v.as_str());

            if let Some(tv) = target_var {
                let rule_type = rule.get("type").or_else(|| rule.get("source")).and_then(|t| t.as_str()).unwrap_or("json");
                let expr = rule.get("expression").or_else(|| rule.get("expr")).or_else(|| rule.get("path")).and_then(|e| e.as_str()).unwrap_or("");
                let enabled = rule.get("enabled").and_then(|b| b.as_bool()).unwrap_or(true);

                extract_rules.push(serde_json::json!({
                    "id": Uuid::new_v4().to_string(),
                    "type": rule_type,
                    "targetVariable": tv,
                    "expression": expr,
                    "enabled": enabled
                }));
            }
        }
    }
    let extract_rules_json = serde_json::to_string(&extract_rules).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT INTO requests (id, collection_id, name, method, url, headers_json, params_json, body_type, body_content, extract_rules_json, description, order_index, created_at_ms, updated_at_ms)
         VALUES (?, ?, ?, ?, ?, ?, '[]', ?, ?, ?, ?, ?, ?, ?)",
        params![
            req_id,
            col_id,
            name,
            method,
            raw_url,
            headers_json,
            body_type,
            body_content,
            extract_rules_json,
            desc,
            order_idx,
            ts,
            ts
        ],
    ).map_err(|e| e.to_string())?;

    *req_count += 1;
    Ok(())
}
