use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use crate::proxy;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RepeaterGroup {
    pub id: String,
    pub name: String,
    pub order_index: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RepeaterRequest {
    pub id: String,
    pub name: String,
    pub group_id: Option<String>,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub response: Option<proxy::Traffic>,
    pub hit_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRepeaterItem {
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub group_id: Option<String>,
    pub response: Option<RepeaterResponse>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RepeaterResponse {
    #[serde(alias = "status", alias = "status_code")]
    pub status: Option<u16>,
    #[serde(alias = "headers", alias = "response_headers")]
    pub headers: Option<Vec<(String, String)>>,
    #[serde(alias = "body", alias = "response_body")]
    pub body: Option<String>,
}

fn get_db_path(app_handle: &AppHandle) -> std::path::PathBuf {
    app_handle.path().app_data_dir().expect("Failed to get app data dir").join("mitm.db")
}

#[tauri::command]
pub async fn create_repeater_item(app_handle: AppHandle, item: CreateRepeaterItem) -> Result<String, String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let id = Uuid::new_v4().to_string();
    let headers_json = serde_json::to_string(&item.headers).unwrap_or_default();
    
    let (res_status, res_headers, res_body) = if let Some(res) = &item.response {
        let status = res.status;
        let headers_json = res.headers.as_ref().map(|h| serde_json::to_string(h).unwrap_or_default());
        let body = res.body.clone();
        (status, headers_json, body)
    } else {
        (None, None, None)
    };

    conn.execute(
        "INSERT INTO repeater_requests (id, name, method, url, headers, body, response_status, response_headers, response_body, group_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![id, item.name, item.method, item.url, headers_json, item.body, res_status, res_headers, res_body, item.group_id],
    ).map_err(|e| e.to_string())?;

    Ok(id)
}

#[tauri::command]
pub async fn update_repeater_request(app_handle: AppHandle, id: String, updates: RepeaterRequest) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let headers_json = serde_json::to_string(&updates.headers).unwrap_or_default();
    
    conn.execute(
        "UPDATE repeater_requests SET name = ?, method = ?, url = ?, headers = ?, body = ?, group_id = ? WHERE id = ?",
        rusqlite::params![updates.name, updates.method, updates.url, headers_json, updates.body, updates.group_id, id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn delete_repeater_request(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM repeater_requests WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn create_repeater_group(app_handle: AppHandle, name: String) -> Result<String, String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    conn.execute("INSERT INTO repeater_groups (id, name) VALUES (?, ?)", [id.clone(), name]).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub async fn delete_repeater_group(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM repeater_groups WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn reorder_repeater_requests(app_handle: AppHandle, ids: Vec<String>) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    for (idx, id) in ids.iter().enumerate() {
        conn.execute("UPDATE repeater_requests SET order_index = ? WHERE id = ?", rusqlite::params![idx as i32, id]).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn reorder_repeater_groups(app_handle: AppHandle, ids: Vec<String>) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    for (idx, id) in ids.iter().enumerate() {
        conn.execute("UPDATE repeater_groups SET order_index = ? WHERE id = ?", rusqlite::params![idx as i32, id]).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn rename_repeater_group(app_handle: AppHandle, id: String, name: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("UPDATE repeater_groups SET name = ? WHERE id = ?", [name, id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn manage_group_assignment(
    app_handle: AppHandle,
    action: String,
    group_id: Option<String>,
    env_id: Option<String>,
    target_env_id: Option<String>,
    bulk_links: Option<Vec<(String, String)>>,
    bulk_unlinks: Option<Vec<(String, String)>>
) -> Result<serde_json::Value, String> {
    let db_path = get_db_path(&app_handle);
    let mut conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;

    match action.as_str() {
        "link" => {
            if let (Some(gid), Some(eid)) = (group_id, env_id) {
                conn.execute("INSERT OR IGNORE INTO environment_groups (group_id, environment_id) VALUES (?, ?)", [gid, eid]).map_err(|e| e.to_string())?;
            }
        },
        "unlink" => {
            if let (Some(gid), Some(eid)) = (group_id, env_id) {
                conn.execute("DELETE FROM environment_groups WHERE group_id = ? AND environment_id = ?", [gid, eid]).map_err(|e| e.to_string())?;
            }
        },
        "move" => {
            if let (Some(gid), Some(eid), Some(teid)) = (group_id, env_id, target_env_id) {
                conn.execute("UPDATE environment_groups SET environment_id = ? WHERE group_id = ? AND environment_id = ?", [teid, gid, eid]).map_err(|e| e.to_string())?;
            }
        },
        "get_assignments" => {
            if let Some(gid) = group_id {
                let mut stmt = conn.prepare("SELECT environment_id FROM environment_groups WHERE group_id = ?").map_err(|e| e.to_string())?;
                let env_ids: Vec<String> = stmt.query_map([gid], |row| row.get(0)).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
                return Ok(serde_json::to_value(env_ids).unwrap());
            }
        },
        "get_groups_for_env" => {
            if let Some(eid) = env_id {
                let mut stmt = conn.prepare("SELECT group_id FROM environment_groups WHERE environment_id = ?").map_err(|e| e.to_string())?;
                let group_ids: Vec<String> = stmt.query_map([eid], |row| row.get(0)).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
                return Ok(serde_json::to_value(group_ids).unwrap());
            }
        },
        "bulk" => {
            let tx = conn.transaction().map_err(|e| e.to_string())?;
            if let Some(links) = bulk_links {
                for (gid, eid) in links {
                    tx.execute("INSERT OR IGNORE INTO environment_groups (group_id, environment_id) VALUES (?, ?)", [gid, eid]).map_err(|e| e.to_string())?;
                }
            }
            if let Some(unlinks) = bulk_unlinks {
                for (gid, eid) in unlinks {
                    tx.execute("DELETE FROM environment_groups WHERE group_id = ? AND environment_id = ?", [gid, eid]).map_err(|e| e.to_string())?;
                }
            }
            tx.commit().map_err(|e| e.to_string())?;
        },
        _ => return Err("Invalid action".to_string()),
    }

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub async fn get_repeater_history(app_handle: AppHandle, repeater_id: String) -> Result<serde_json::Value, String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, method, url, request, response, timestamp FROM repeater_history WHERE repeater_id = ? ORDER BY timestamp DESC").map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([repeater_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, String>(0)?,
            "method": row.get::<_, String>(1)?,
            "url": row.get::<_, String>(2)?,
            "request": row.get::<_, String>(3)?,
            "response": row.get::<_, String>(4)?,
            "timestamp": row.get::<_, i64>(5)?
        }))
    }).map_err(|e| e.to_string())?;

    let mut history = Vec::new();
    for row in rows {
        if let Ok(h) = row { history.push(h); }
    }
    Ok(serde_json::to_value(history).unwrap())
}

#[tauri::command]
pub async fn clear_repeater_history(app_handle: AppHandle, repeater_id: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM repeater_history WHERE repeater_id = ?", [repeater_id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_repeater_history_item(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM repeater_history WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ImportRepeaterData {
    pub name: Option<String>,
    pub url: Option<String>,
    pub header: Option<serde_json::Value>,
    pub placeholders: Option<serde_json::Value>,
    pub all_environments: Option<Vec<serde_json::Value>>,
    pub all_variables: Option<Vec<serde_json::Value>>,
    pub test_cases: Option<Vec<serde_json::Value>>,
    pub import_environments: Option<Vec<String>>,
    pub import_groups: Option<Vec<String>>,
    pub link_to_environment: Option<String>,
    pub link_to_environments: Option<Vec<String>>,
    pub smart_link: Option<bool>,
}

#[tauri::command]
pub async fn import_repeater_data(app_handle: AppHandle, data: ImportRepeaterData) -> Result<serde_json::Value, String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let mut imported_envs = 0;
    let mut imported_vars = 0;
    let mut imported_groups = 0;
    let mut imported_requests = 0;
    
    let mut newly_imported_env_ids = Vec::new();

    // 1. Import environments and variables if requested
    if let (Some(envs), Some(import_env_ids)) = (&data.all_environments, &data.import_environments) {
        for env in envs {
            let env_obj = env.as_object().ok_or("Invalid environment object")?;
            let env_id = env_obj.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| Uuid::new_v4().to_string());
            let env_name = env_obj.get("name").and_then(|v| v.as_str()).unwrap_or("Imported Environment").to_string();

            // Check if this environment should be imported
            if !import_env_ids.contains(&env_id) && !import_env_ids.contains(&env_name) {
                continue;
            }

            let new_env_id = Uuid::new_v4().to_string();
            newly_imported_env_ids.push(new_env_id.clone());
            conn.execute(
                "INSERT INTO environments (id, name, is_active) VALUES (?, ?, 0)",
                rusqlite::params![new_env_id, env_name],
            ).map_err(|e| e.to_string())?;
            
            imported_envs += 1;

            // Import variables for this environment
            if let Some(vars) = &data.all_variables {
                for var in vars {
                    let var_obj = var.as_object().ok_or("Invalid variable object")?;
                    let var_env_id = var_obj.get("environmentId").and_then(|v| v.as_str()).unwrap_or("");
                    
                    if var_env_id == env_id.as_str() {
                        let var_name = var_obj.get("name").and_then(|v| v.as_str()).unwrap_or("unnamed").to_string();
                        let active_index = var_obj.get("activeIndex").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let values = var_obj.get("values").and_then(|v| v.as_array()).ok_or("Invalid values array")?;

                        let new_var_id = Uuid::new_v4().to_string();
                        conn.execute(
                            "INSERT INTO variables (id, environment_id, name, active_index) VALUES (?, ?, ?, ?)",
                            rusqlite::params![new_var_id, new_env_id, var_name, active_index],
                        ).map_err(|e| e.to_string())?;

                        for (val_idx, value_obj) in values.iter().enumerate() {
                            if let Some(v_obj) = value_obj.as_object() {
                                let value_name = v_obj.get("name").and_then(|v| v.as_str()).unwrap_or(&format!("Value {}", val_idx)).to_string();
                                let value_str = v_obj.get("value").and_then(|v| v.as_str()).unwrap_or("");
                                
                                let val_id = Uuid::new_v4().to_string();
                                conn.execute(
                                    "INSERT INTO variable_values (id, variable_id, name, value) VALUES (?, ?, ?, ?)",
                                    rusqlite::params![val_id, new_var_id, value_name, value_str],
                                ).map_err(|e| e.to_string())?;
                                
                                imported_vars += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Import repeater groups and requests
    if let Some(test_cases) = &data.test_cases {
        let import_group_names = data.import_groups.clone().unwrap_or_default();
        let link_env_id = data.link_to_environment.clone();
        let mut target_env_ids = data.link_to_environments.clone().unwrap_or_default();
        
        // Add single link_to_environment for backwards compatibility
        if let Some(id) = link_env_id {
            if !target_env_ids.contains(&id) {
                target_env_ids.push(id);
            }
        }
        
        // Add newly imported environments if smart_link is true
        if data.smart_link.unwrap_or(false) {
            for id in &newly_imported_env_ids {
                if !target_env_ids.contains(id) {
                    target_env_ids.push(id.clone());
                }
            }
        }

        for (group_idx, tc) in test_cases.iter().enumerate() {
            let tc_obj = tc.as_object().ok_or("Invalid test case object")?;
            let group_name = tc_obj.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| format!("Imported Group {}", group_idx));
            
            // Check if this group should be imported
            if !import_group_names.is_empty() && !import_group_names.contains(&group_name) {
                continue;
            }

            let new_group_id = Uuid::new_v4().to_string();
            let global_url = data.url.as_deref().unwrap_or("");
            let group_url = tc_obj.get("url").and_then(|v| v.as_str()).unwrap_or(global_url);

            conn.execute(
                "INSERT INTO repeater_groups (id, name, order_index) VALUES (?, ?, ?)",
                rusqlite::params![new_group_id, group_name, group_idx],
            ).map_err(|e| e.to_string())?;

            imported_groups += 1;

            // Link group to environments
            for env_id in &target_env_ids {
                conn.execute(
                    "INSERT OR IGNORE INTO environment_groups (group_id, environment_id) VALUES (?, ?)",
                    rusqlite::params![new_group_id, env_id],
                ).map_err(|e| e.to_string())?;
            }

            // Import requests in this group
            if let Some(targets) = tc_obj.get("target").and_then(|v| v.as_array()) {
                for (req_idx, target) in targets.iter().enumerate() {
                    let target_obj = target.as_object().ok_or("Invalid target object")?;
                    let req_name = target_obj.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| format!("Request {}", req_idx));
                    let method = target_obj.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
                    let endpoint = target_obj.get("endpoint").and_then(|v| v.as_str()).unwrap_or("");
                    let params = target_obj.get("params").and_then(|v| v.as_object());
                    
                    // Build full URL
                    let mut full_url = group_url.to_string() + endpoint;
                    if let Some(p) = params {
                        let query_parts: Vec<String> = p.iter()
                            .map(|(k, v)| {
                                let val = v.as_str().unwrap_or("");
                                format!("{}={}", k, val)
                            })
                            .collect();
                        if !query_parts.is_empty() {
                            full_url.push('?');
                            full_url.push_str(&query_parts.join("&"));
                        }
                    }

                    // Merge headers
                    let mut headers: Vec<(String, String)> = Vec::new();
                    if let Some(global_header) = &data.header {
                        if let Some(header_obj) = global_header.as_object() {
                            for (k, v) in header_obj {
                                if let Some(val) = v.as_str() {
                                    headers.push((k.clone(), val.to_string()));
                                }
                            }
                        }
                    }
                    if let Some(target_headers) = target_obj.get("header").and_then(|v| v.as_object()) {
                        for (k, v) in target_headers {
                            if let Some(val) = v.as_str() {
                                // Replace or add header
                                if let Some(existing) = headers.iter_mut().find(|(hk, _)| hk == k) {
                                    existing.1 = val.to_string();
                                } else {
                                    headers.push((k.clone(), val.to_string()));
                                }
                            } else if v.is_null() {
                                // Remove header
                                headers.retain(|(hk, _)| hk != k);
                            }
                        }
                    }

                    let body = target_obj.get("body").and_then(|v| v.as_str()).unwrap_or("");
                    let extract = target_obj.get("extract").and_then(|v| serde_json::to_string(v).ok()).unwrap_or_else(|| "{}".to_string());

                    let headers_json = serde_json::to_string(&headers).unwrap_or_default();
                    let req_id = Uuid::new_v4().to_string();

                    conn.execute(
                        "INSERT INTO repeater_requests (id, name, group_id, method, url, headers, body, extract, order_index) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                        rusqlite::params![req_id, req_name, new_group_id, method, full_url, headers_json, body, extract, req_idx],
                    ).map_err(|e| e.to_string())?;

                    imported_requests += 1;
                }
            }
        }
    }

    conn.execute("PRAGMA optimize", []).ok();

    Ok(serde_json::json!({
        "success": true,
        "imported": {
            "environments": imported_envs,
            "variables": imported_vars,
            "groups": imported_groups,
            "requests": imported_requests
        }
    }))
}
