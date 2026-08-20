use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use crate::proxy;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RepeaterGroup {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub order_index: i32,
    pub extract: Option<serde_json::Value>,
    pub description: Option<String>,
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
    pub extract: Option<serde_json::Value>,
    pub response: Option<proxy::Traffic>,
    pub hit_count: i32,
    pub description: Option<String>,
    pub body_mode: Option<String>,
    pub body_json: Option<String>,
    pub body_urlencoded: Option<String>,
    pub body_multipart: Option<String>,
    pub url_params: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRepeaterItem {
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub extract: Option<serde_json::Value>,
    pub group_id: Option<String>,
    pub response: Option<RepeaterResponse>,
    pub description: Option<String>,
    pub body_mode: Option<String>,
    pub body_json: Option<String>,
    pub body_urlencoded: Option<String>,
    pub body_multipart: Option<String>,
    pub url_params: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRepeaterItem {
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub extract: Option<serde_json::Value>,
    pub group_id: Option<String>,
    pub description: Option<String>,
    pub body_mode: Option<String>,
    pub body_json: Option<String>,
    pub body_urlencoded: Option<String>,
    pub body_multipart: Option<String>,
    pub url_params: Option<String>,
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
    let extract_json: Option<String> = item.extract.as_ref().and_then(|v| serde_json::to_string(v).ok());
    
    let (res_status, res_headers, res_body) = if let Some(res) = &item.response {
        let status = res.status;
        let headers_json = res.headers.as_ref().map(|h| serde_json::to_string(h).unwrap_or_default());
        let body = res.body.clone();
        (status, headers_json, body)
    } else {
        (None, None, None)
    };

    let min_order: i32 = if let Some(ref gid) = item.group_id {
        conn.query_row(
            "SELECT COALESCE(MIN(order_index), 0) FROM repeater_requests WHERE group_id = ?",
            [gid],
            |row| row.get(0)
        ).unwrap_or(0)
    } else {
        conn.query_row(
            "SELECT COALESCE(MIN(order_index), 0) FROM repeater_requests WHERE group_id IS NULL",
            [],
            |row| row.get(0)
        ).unwrap_or(0)
    };
    let new_order_index = min_order - 1;

    conn.execute(
        "INSERT INTO repeater_requests (id, name, method, url, headers, body, extract, response_status, response_headers, response_body, group_id, order_index, description, url_params) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![id, item.name, item.method, item.url, headers_json, item.body, extract_json, res_status, res_headers, res_body, item.group_id, new_order_index, item.description, item.url_params],
    ).map_err(|e| e.to_string())?;

    let mode = item.body_mode.as_deref().unwrap_or("raw");
    conn.execute(
        "INSERT OR REPLACE INTO request_bodies (request_id, body_mode, body_raw, body_json, body_urlencoded, body_multipart) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params![id, mode, item.body, item.body_json, item.body_urlencoded, item.body_multipart],
    ).map_err(|e| e.to_string())?;

    Ok(id)
}

#[tauri::command]
pub async fn update_repeater_request(app_handle: AppHandle, id: String, updates: UpdateRepeaterItem) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let headers_json = serde_json::to_string(&updates.headers).unwrap_or_default();
    let extract_json: Option<String> = updates.extract.as_ref().and_then(|v| serde_json::to_string(v).ok());
    
    conn.execute(
        "UPDATE repeater_requests SET name = ?, method = ?, url = ?, headers = ?, body = ?, extract = ?, group_id = ?, description = ?, url_params = ? WHERE id = ?",
        rusqlite::params![updates.name, updates.method, updates.url, headers_json, updates.body, extract_json, updates.group_id, updates.description, updates.url_params, id],
    ).map_err(|e| e.to_string())?;

    let mode = updates.body_mode.as_deref().unwrap_or("raw");
    conn.execute(
        "INSERT OR REPLACE INTO request_bodies (request_id, body_mode, body_raw, body_json, body_urlencoded, body_multipart) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params![id, mode, updates.body, updates.body_json, updates.body_urlencoded, updates.body_multipart],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn update_repeater_group_description(app_handle: AppHandle, id: String, description: Option<String>) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("UPDATE repeater_groups SET description = ? WHERE id = ?", rusqlite::params![description, id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_repeater_request(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM repeater_requests WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

fn delete_group_and_descendants(conn: &rusqlite::Connection, group_id: &str) -> Result<(), String> {
    let mut stmt = conn.prepare("SELECT id FROM repeater_groups WHERE parent_id = ?").map_err(|e| e.to_string())?;
    let child_ids: Vec<String> = stmt.query_map([group_id], |row| row.get(0)).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
    for cid in &child_ids {
        delete_group_and_descendants(conn, cid)?;
    }
    conn.execute("DELETE FROM repeater_requests WHERE group_id = ?", [group_id]).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM environment_groups WHERE group_id = ?", [group_id]).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM repeater_groups WHERE id = ?", [group_id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn create_repeater_group(app_handle: AppHandle, name: String, parent_id: Option<String>) -> Result<String, String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    conn.execute("INSERT INTO repeater_groups (id, parent_id, name) VALUES (?, ?, ?)", rusqlite::params![id, parent_id, name]).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub async fn delete_repeater_group(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    delete_group_and_descendants(&conn, &id)?;
    Ok(())
}

#[tauri::command]
pub async fn bulk_delete_repeater_groups(app_handle: AppHandle, ids: Vec<String>) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    for id in ids {
        delete_group_and_descendants(&conn, &id)?;
    }
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
pub async fn update_repeater_group_extractions(app_handle: AppHandle, id: String, extract: serde_json::Value) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    let extract_json = serde_json::to_string(&extract).map_err(|e| e.to_string())?;
    conn.execute("UPDATE repeater_groups SET extract = ? WHERE id = ?", rusqlite::params![extract_json, id]).map_err(|e| e.to_string())?;
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

                        let new_var_id = Uuid::new_v4().to_string();
                        conn.execute(
                            "INSERT INTO variables (id, environment_id, name, active_index) VALUES (?, ?, ?, ?)",
                            rusqlite::params![new_var_id, new_env_id, var_name, active_index],
                        ).map_err(|e| e.to_string())?;

                        let values_opt = var_obj.get("values").and_then(|v| v.as_array());
                        let mut has_auto_variant = false;

                        if let Some(values) = values_opt {
                            for (_val_idx, value_obj) in values.iter().enumerate() {
                                if let Some(v_obj) = value_obj.as_object() {
                                    let value_name = v_obj.get("name").and_then(|v| v.as_str()).unwrap_or("(auto)").to_string();
                                    if value_name == "(auto)" {
                                        has_auto_variant = true;
                                    }
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

                        // ALWAYS ensure there is an (auto) variant created for this variable
                        if !has_auto_variant {
                            let val_id = Uuid::new_v4().to_string();
                            let default_val = var_obj.get("value").and_then(|v| v.as_str()).unwrap_or("");
                            conn.execute(
                                "INSERT INTO variable_values (id, variable_id, name, value) VALUES (?, ?, ?, ?)",
                                rusqlite::params![val_id, new_var_id, "(auto)", default_val],
                            ).map_err(|e| e.to_string())?;
                            imported_vars += 1;
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

        fn import_group_recursive(
            conn: &rusqlite::Connection,
            tc_obj: &serde_json::Map<String, serde_json::Value>,
            parent_id: Option<String>,
            group_idx: usize,
            global_url: &str,
            global_header: Option<&serde_json::Value>,
            target_env_ids: &[String],
            import_group_names: &[String],
            imported_groups: &mut i32,
            imported_requests: &mut i32,
        ) -> Result<(), String> {
            let group_name = tc_obj.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| format!("Imported Group {}", group_idx));
            
            // Check if this group should be imported (selective import at root level)
            if parent_id.is_none() && !import_group_names.is_empty() && !import_group_names.contains(&group_name) {
                return Ok(());
            }

            let new_group_id = Uuid::new_v4().to_string();
            let group_url = tc_obj.get("url").and_then(|v| v.as_str()).unwrap_or(global_url);
            let group_desc = tc_obj.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());

            conn.execute(
                "INSERT INTO repeater_groups (id, parent_id, name, order_index, description) VALUES (?, ?, ?, ?, ?)",
                rusqlite::params![new_group_id, parent_id, group_name, group_idx as i32, group_desc],
            ).map_err(|e| e.to_string())?;

            *imported_groups += 1;

            // Link group to environments
            for env_id in target_env_ids {
                conn.execute(
                    "INSERT OR IGNORE INTO environment_groups (group_id, environment_id) VALUES (?, ?)",
                    rusqlite::params![new_group_id, env_id],
                ).map_err(|e| e.to_string())?;
            }

            // Import direct request targets
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
                    if let Some(gh) = global_header {
                        if let Some(header_obj) = gh.as_object() {
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
                                if let Some(existing) = headers.iter_mut().find(|(hk, _)| hk == k) {
                                    existing.1 = val.to_string();
                                } else {
                                    headers.push((k.clone(), val.to_string()));
                                }
                            } else if v.is_null() {
                                headers.retain(|(hk, _)| hk != k);
                            }
                        }
                    }

                    let body = match target_obj.get("body") {
                        Some(v) if v.is_string() => v.as_str().unwrap_or("").to_string(),
                        Some(v) if !v.is_null() => serde_json::to_string(v).unwrap_or_default(),
                        _ => String::new(),
                    };
                    let extract = target_obj.get("extract").and_then(|v| serde_json::to_string(v).ok()).unwrap_or_else(|| "{}".to_string());
                    let req_desc = target_obj.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let url_params = target_obj.get("url_params").and_then(|v| if v.is_string() { v.as_str().map(|s| s.to_string()) } else { serde_json::to_string(v).ok() });

                    let body_mode = target_obj.get("body_mode").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| {
                        if body.trim().starts_with('{') || body.trim().starts_with('[') {
                            "json".to_string()
                        } else if body.contains('=') && !body.trim().starts_with('{') {
                            "urlencoded".to_string()
                        } else if body.contains("__form_data") {
                            "multipart".to_string()
                        } else {
                            "raw".to_string()
                        }
                    });

                    let body_json = target_obj.get("body_json").and_then(|v| if v.is_string() { v.as_str().map(|s| s.to_string()) } else { serde_json::to_string(v).ok() }).or_else(|| if body_mode == "json" && !body.is_empty() { Some(body.clone()) } else { None });
                    let body_urlencoded = target_obj.get("body_urlencoded").and_then(|v| v.as_str().map(|s| s.to_string())).or_else(|| if body_mode == "urlencoded" && !body.is_empty() { Some(body.clone()) } else { None });
                    let body_multipart = target_obj.get("body_multipart").and_then(|v| if v.is_string() { v.as_str().map(|s| s.to_string()) } else { serde_json::to_string(v).ok() }).or_else(|| if body_mode == "multipart" && !body.is_empty() { Some(body.clone()) } else { None });

                    let headers_json = serde_json::to_string(&headers).unwrap_or_default();
                    let req_id = Uuid::new_v4().to_string();

                    conn.execute(
                        "INSERT INTO repeater_requests (id, name, group_id, method, url, headers, body, extract, order_index, description, url_params) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                        rusqlite::params![req_id, req_name, new_group_id, method, full_url, headers_json, body, extract, req_idx as i32, req_desc, url_params],
                    ).map_err(|e| e.to_string())?;

                    conn.execute(
                        "INSERT OR REPLACE INTO request_bodies (request_id, body_mode, body_raw, body_json, body_urlencoded, body_multipart) VALUES (?, ?, ?, ?, ?, ?)",
                        rusqlite::params![req_id, body_mode, body, body_json, body_urlencoded, body_multipart],
                    ).map_err(|e| e.to_string())?;

                    *imported_requests += 1;
                }
            }

            // Import subfolders recursively
            if let Some(sub_folders) = tc_obj.get("folders").and_then(|v| v.as_array()) {
                for (sub_idx, sub_f) in sub_folders.iter().enumerate() {
                    if let Some(sub_obj) = sub_f.as_object() {
                        import_group_recursive(
                            conn,
                            sub_obj,
                            Some(new_group_id.clone()),
                            sub_idx,
                            group_url,
                            global_header,
                            target_env_ids,
                            import_group_names,
                            imported_groups,
                            imported_requests,
                        )?;
                    }
                }
            }

            Ok(())
        }

        let global_url = data.url.as_deref().unwrap_or("");

        for (group_idx, tc) in test_cases.iter().enumerate() {
            if let Some(tc_obj) = tc.as_object() {
                import_group_recursive(
                    &conn,
                    tc_obj,
                    None,
                    group_idx,
                    global_url,
                    data.header.as_ref(),
                    &target_env_ids,
                    &import_group_names,
                    &mut imported_groups,
                    &mut imported_requests,
                )?;
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
