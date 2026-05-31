use tauri::{AppHandle, Manager};
use rusqlite::OptionalExtension;
use crate::db;
use crate::proxy;
use crate::models::{SyncData, Environment, GlobalVariable, VariableValue, Replacement};
use crate::repeater::{RepeaterGroup, RepeaterRequest};


#[tauri::command]
pub async fn update_prefs(app_handle: AppHandle, prefs: serde_json::Value) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let prefs_json = serde_json::to_string(&prefs).map_err(|e| e.to_string())?;
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO app_state (key, value) VALUES ('prefs', ?)", [prefs_json]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn update_ui_layout(app_handle: AppHandle, layout: serde_json::Value) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let layout_json = serde_json::to_string(&layout).map_err(|e| e.to_string())?;
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO app_state (key, value) VALUES ('ui_layout', ?)", [layout_json]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn save_state(app_handle: AppHandle, key: String, value: String) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO app_state (key, value) VALUES (?, ?)", [key, value]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn purge_all_data(app_handle: AppHandle) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         DELETE FROM history;
         DELETE FROM repeater_history;
         DELETE FROM repeater_requests;
         DELETE FROM repeater_groups;
         DELETE FROM environments;
         DELETE FROM variables;
         DELETE FROM variable_values;
         DELETE FROM environment_groups;
         DELETE FROM replacements;
         DELETE FROM app_state WHERE key NOT IN ('proxy_config','filter_config');"
    ).map_err(|e| e.to_string())?;

    if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
        let uploads_dir = app_data_dir.join("uploads");
        if uploads_dir.exists() {
            let _ = std::fs::remove_dir_all(uploads_dir);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn upload_file(app_handle: AppHandle, name: String, content: Vec<u8>) -> Result<String, String> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let uploads_dir = app_data_dir.join("uploads");
    if !uploads_dir.exists() {
        std::fs::create_dir_all(&uploads_dir).map_err(|e| e.to_string())?;
    }
    
    let file_id = uuid::Uuid::new_v4().to_string();
    let extension = std::path::Path::new(&name).extension().and_then(|s| s.to_str()).unwrap_or("bin");
    let file_name = format!("{}.{}", file_id, extension);
    let dest_path = uploads_dir.join(&file_name);
    
    std::fs::write(&dest_path, content).map_err(|e| e.to_string())?;
    
    Ok(dest_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn get_repeater_requests(app_handle: AppHandle, group_id: String) -> Result<Vec<RepeaterRequest>, String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;

    let (query, params): (&str, Vec<rusqlite::types::Value>) = if group_id == "All" {
        ("SELECT id, name, group_id, method, url, headers, body, extract, response_status, response_headers, response_body, hit_count FROM repeater_requests ORDER BY order_index", vec![])
    } else if group_id == "null" {
        ("SELECT id, name, group_id, method, url, headers, body, extract, response_status, response_headers, response_body, hit_count FROM repeater_requests WHERE group_id IS NULL ORDER BY order_index", vec![])
    } else {
        ("SELECT id, name, group_id, method, url, headers, body, extract, response_status, response_headers, response_body, hit_count FROM repeater_requests WHERE group_id = ? ORDER BY order_index", vec![rusqlite::types::Value::Text(group_id)])
    };

    let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;
    let repeater_requests = stmt.query_map(rusqlite::params_from_iter(params), |row| {
        let headers: Vec<(String, String)> = serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default();
        let extract: Option<serde_json::Value> = row.get::<_, Option<String>>(7)?.and_then(|s| serde_json::from_str(&s).ok());
        let res_status: Option<u16> = row.get(8).ok();
        let response = if let Some(status) = res_status {
             let res_headers: Vec<(String, String)> = serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default();
             Some(proxy::Traffic {
                 id: row.get::<_, String>(0)? + "_res",
                 method: row.get(3)?,
                 url: row.get(4)?,
                 host: String::new(),
                 status_code: status,
                 request_headers: headers.clone(),
                 response_headers: res_headers,
                 request_body: row.get(6)?,
                 response_body: row.get(10)?,
                 phase: "response".to_string(),
                 is_intercepted: false,
                 intercepted_at: None,
             })
        } else { None };

        Ok(RepeaterRequest {
            id: row.get(0)?,
            name: row.get(1)?,
            group_id: row.get(2)?,
            method: row.get(3)?,
            url: row.get(4)?,
            headers,
            body: row.get(6)?,
            extract,
            response,
            hit_count: row.get(11)?,
        })
    }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    Ok(repeater_requests)
}

#[tauri::command]
pub async fn sync_data(app_handle: AppHandle) -> Result<SyncData, String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;

    // 1. History
    let mut stmt = conn.prepare("SELECT id, method, url, host, status_code, request_headers, response_headers, request_body, response_body, phase FROM history ORDER BY created_at DESC LIMIT 500").map_err(|e| e.to_string())?;
    let history = stmt.query_map([], |row| {
        let req_headers: Vec<(String, String)> = serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default();
        let res_headers: Vec<(String, String)> = serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default();
        Ok(proxy::Traffic {
            id: row.get(0)?, method: row.get(1)?, url: row.get(2)?, host: row.get(3)?, status_code: row.get(4)?,
            request_headers: req_headers, response_headers: res_headers,
            request_body: row.get(7)?, response_body: row.get(8)?, phase: row.get(9)?,
            is_intercepted: false, intercepted_at: None,
        })
    }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    // 2. Determine active environment
    let active_env_id: Option<String> = conn.prepare("SELECT id FROM environments WHERE is_active = 1 LIMIT 1")
        .map_err(|e| e.to_string())?
        .query_row([], |row| row.get(0))
        .optional()
        .map_err(|e| e.to_string())?;

    let mut stmt = if active_env_id.is_some() {
        conn.prepare("SELECT id, name, order_index FROM repeater_groups WHERE id IN (SELECT group_id FROM environment_groups WHERE environment_id = ?) ORDER BY order_index")
            .map_err(|e| e.to_string())?
    } else {
        conn.prepare("SELECT id, name, order_index FROM repeater_groups ORDER BY order_index").map_err(|e| e.to_string())?
    };

    let repeater_groups = if let Some(ref env_id) = active_env_id {
        stmt.query_map([env_id], |row| {
            Ok(RepeaterGroup { id: row.get(0)?, name: row.get(1)?, order_index: row.get(2)? })
        }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect()
    } else {
        stmt.query_map([], |row| {
            Ok(RepeaterGroup { id: row.get(0)?, name: row.get(1)?, order_index: row.get(2)? })
        }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect()
    };

    // 2.5. Determine active group
    let active_group_id: Option<String> = conn.query_row("SELECT value FROM app_state WHERE key = 'active_group_id'", [], |row| row.get(0))
        .optional()
        .unwrap_or(None);

    let effective_group_id = active_group_id.clone().unwrap_or_else(|| "null".to_string());

    // 3. Repeater Requests (filtered by effective_group_id)
    let (request_query, request_params): (&str, Vec<rusqlite::types::Value>) = if effective_group_id == "All" {
        ("SELECT id, name, group_id, method, url, headers, body, extract, response_status, response_headers, response_body, hit_count FROM repeater_requests ORDER BY order_index", vec![])
    } else if effective_group_id == "null" {
        ("SELECT id, name, group_id, method, url, headers, body, extract, response_status, response_headers, response_body, hit_count FROM repeater_requests WHERE group_id IS NULL ORDER BY order_index", vec![])
    } else {
        ("SELECT id, name, group_id, method, url, headers, body, extract, response_status, response_headers, response_body, hit_count FROM repeater_requests WHERE group_id = ? ORDER BY order_index", vec![rusqlite::types::Value::Text(effective_group_id)])
    };

    let mut stmt = conn.prepare(request_query).map_err(|e| e.to_string())?;
    let repeater_requests = stmt.query_map(rusqlite::params_from_iter(request_params), |row| {
        let headers: Vec<(String, String)> = serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default();
        let extract: Option<serde_json::Value> = row.get::<_, Option<String>>(7)?.and_then(|s| serde_json::from_str(&s).ok());
        let res_status: Option<u16> = row.get(8).ok();
        let response = if let Some(status) = res_status {
             let res_headers: Vec<(String, String)> = serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default();
             Some(proxy::Traffic {
                 id: row.get::<_, String>(0)? + "_res",
                 method: row.get(3)?,
                 url: row.get(4)?,
                 host: String::new(),
                 status_code: status,
                 request_headers: headers.clone(),
                 response_headers: res_headers,
                 request_body: row.get(6)?,
                 response_body: row.get(10)?,
                 phase: "response".to_string(),
                 is_intercepted: false,
                 intercepted_at: None,
             })
        } else { None };

        Ok(RepeaterRequest {
            id: row.get(0)?,
            name: row.get(1)?,
            group_id: row.get(2)?,
            method: row.get(3)?,
            url: row.get(4)?,
            headers,
            body: row.get(6)?,
            extract,
            response,
            hit_count: row.get(11)?,
        })
    }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    // 4. Environments
    let mut stmt = conn.prepare("SELECT id, name, is_active FROM environments").map_err(|e| e.to_string())?;
    let environments = stmt.query_map([], |row| {
        Ok(Environment { id: row.get(0)?, name: row.get(1)?, is_active: row.get::<_, i32>(2)? != 0 })
    }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    // 5. Variables
    let mut stmt = conn.prepare("SELECT id, environment_id, name, active_index, order_index FROM variables ORDER BY order_index").map_err(|e| e.to_string())?;
    let variables = stmt.query_map([], |row| {
        let var_id: String = row.get(0)?;
        let mut val_stmt = conn.prepare("SELECT id, name, value FROM variable_values WHERE variable_id = ?").unwrap();
        let values = val_stmt.query_map([&var_id], |vrow| {
            Ok(VariableValue { id: vrow.get(0)?, name: vrow.get(1)?, value: vrow.get(2)? })
        }).unwrap().filter_map(|r| r.ok()).collect();

        Ok(GlobalVariable {
            id: var_id, environment_id: row.get(1)?, name: row.get(2)?, active_index: row.get(3)?, order_index: row.get(4)?,
            values,
        })
    }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    // 6. Replacements
    let mut stmt = conn.prepare("SELECT id, type, pattern, replacement, description, is_active, order_index FROM replacements ORDER BY order_index").map_err(|e| e.to_string())?;
    let replacements = stmt.query_map([], |row| {
        Ok(Replacement {
            id: row.get(0)?, r_type: row.get(1)?, pattern: row.get(2)?, replacement: row.get(3)?,
            description: row.get(4)?, is_active: row.get::<_, i32>(5)? != 0, order_index: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    // 7. Prefs and UI Layout
    let mut prefs = serde_json::Value::Null;
    if let Ok(config_str) = conn.query_row::<String, _, _>("SELECT value FROM app_state WHERE key = 'prefs'", [], |row| row.get(0)) {
        if let Ok(v) = serde_json::from_str(&config_str) { prefs = v; }
    }

    let mut ui_layout = serde_json::Value::Null;
    if let Ok(config_str) = conn.query_row::<String, _, _>("SELECT value FROM app_state WHERE key = 'ui_layout'", [], |row| row.get(0)) {
        if let Ok(v) = serde_json::from_str(&config_str) { ui_layout = v; }
    }

    let mut toolkit_json = String::new();
    if let Ok(v) = conn.query_row::<String, _, _>("SELECT value FROM app_state WHERE key = 'toolkit_json'", [], |row| row.get(0)) {
        toolkit_json = v;
    }

    let mut history_limits = serde_json::Value::Null;
    if let Ok(config_str) = conn.query_row::<String, _, _>("SELECT value FROM app_state WHERE key = 'history_limits'", [], |row| row.get(0)) {
        if let Ok(v) = serde_json::from_str(&config_str) { history_limits = v; }
    }

    let mut filter_config = proxy::FilterConfig::default();
    if let Ok(config_str) = conn.query_row::<String, _, _>("SELECT value FROM app_state WHERE key = 'filter_config'", [], |row| row.get(0)) {
        if let Ok(v) = serde_json::from_str(&config_str) { filter_config = v; }
    }

    Ok(SyncData { history, repeater_groups, repeater_requests, environments, variables, replacements, prefs, ui_layout, toolkit_json, history_limits, active_group_id, filter_config })
}
