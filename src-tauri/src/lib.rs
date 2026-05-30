mod ca;
mod proxy;
mod db;
mod repeater;

use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use serde::{Deserialize, Serialize};

// Re-exports from other modules
pub use repeater::{RepeaterGroup, RepeaterRequest, CreateRepeaterItem};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableValue {
    pub id: String,
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalVariable {
    pub id: String,
    pub environment_id: String,
    pub name: String,
    pub active_index: i32,
    pub order_index: i32,
    pub values: Vec<VariableValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Replacement {
    pub id: String,
    pub r_type: String, 
    pub pattern: String,
    pub replacement: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub order_index: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncData {
    pub history: Vec<proxy::Traffic>,
    pub repeater_groups: Vec<RepeaterGroup>,
    pub repeater_requests: Vec<RepeaterRequest>,
    pub environments: Vec<Environment>,
    pub variables: Vec<GlobalVariable>,
    pub replacements: Vec<Replacement>,
    pub prefs: serde_json::Value,
    pub ui_layout: serde_json::Value,
    pub toolkit_json: String,
    pub history_limits: serde_json::Value,
}

#[tauri::command]
async fn save_state(app_handle: AppHandle, key: String, value: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO app_state (key, value) VALUES (?, ?)", [key, value]).map_err(|e| e.to_string())?;
    Ok(())
}

pub struct AppState {
    pub proxy_manager: Arc<Mutex<ProxyManager>>,
    pub intercept_state: Arc<Mutex<proxy::InterceptState>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyConfig {
    pub bindings: Vec<String>,
    pub enabled: bool,
}

pub struct ProxyManager {
    pub active_listeners: HashMap<String, tokio::sync::oneshot::Sender<()>>,
    pub config: ProxyConfig,
}

fn get_db_path(app_handle: &AppHandle) -> std::path::PathBuf {
    app_handle.path().app_data_dir().expect("Failed to get app data dir").join("mitm.db")
}

fn init_database(app_handle: &AppHandle) -> Result<(), String> {
    let db_path = get_db_path(app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;

    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS history (
            id TEXT PRIMARY KEY,
            method TEXT,
            url TEXT,
            host TEXT,
            status_code INTEGER,
            request_headers TEXT,
            response_headers TEXT,
            request_body TEXT,
            response_body TEXT,
            phase TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS repeater_groups (
            id TEXT PRIMARY KEY,
            name TEXT UNIQUE,
            order_index INTEGER DEFAULT 0,
            timestamp INTEGER
        );
        CREATE TABLE IF NOT EXISTS repeater_requests (
            id TEXT PRIMARY KEY,
            name TEXT,
            group_id TEXT,
            method TEXT,
            url TEXT,
            headers TEXT,
            body TEXT,
            response_status INTEGER,
            response_headers TEXT,
            response_body TEXT,
            order_index INTEGER DEFAULT 0,
            extract TEXT,
            hit_count INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(group_id) REFERENCES repeater_groups(id) ON DELETE SET NULL
        );
        CREATE TABLE IF NOT EXISTS repeater_history (
            id TEXT PRIMARY KEY,
            repeater_id TEXT,
            method TEXT,
            url TEXT,
            request TEXT,
            response TEXT,
            timestamp INTEGER,
            FOREIGN KEY(repeater_id) REFERENCES repeater_requests(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS environments (
            id TEXT PRIMARY KEY,
            name TEXT,
            is_active INTEGER DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS variables (
            id TEXT PRIMARY KEY,
            environment_id TEXT,
            name TEXT,
            active_index INTEGER DEFAULT 0,
            order_index INTEGER DEFAULT 0,
            FOREIGN KEY(environment_id) REFERENCES environments(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS variable_values (
            id TEXT PRIMARY KEY,
            variable_id TEXT,
            name TEXT,
            value TEXT,
            FOREIGN KEY(variable_id) REFERENCES variables(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS environment_groups (
            environment_id TEXT,
            group_id TEXT,
            order_index INTEGER DEFAULT 0,
            PRIMARY KEY (environment_id, group_id),
            FOREIGN KEY(environment_id) REFERENCES environments(id) ON DELETE CASCADE,
            FOREIGN KEY(group_id) REFERENCES repeater_groups(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS app_state (
            key TEXT PRIMARY KEY,
            value TEXT
        );
        CREATE TABLE IF NOT EXISTS replacements (
            id TEXT PRIMARY KEY, 
            type TEXT NOT NULL, 
            pattern TEXT NOT NULL, 
            replacement TEXT NOT NULL,
            description TEXT,
            is_active INTEGER DEFAULT 1,
            order_index INTEGER DEFAULT 0,
            created_at INTEGER DEFAULT (strftime('%s', 'now')),
            updated_at INTEGER DEFAULT (strftime('%s', 'now'))
        );
    ").map_err(|e| e.to_string())?;

    Ok(())
}

async fn spawn_proxy_listener(
    app_handle: AppHandle,
    addr_str: String,
    ca: Arc<ca::CA>,
    intercept_state: Arc<Mutex<proxy::InterceptState>>,
) -> Result<tokio::sync::oneshot::Sender<()>, String> {
    let addr: SocketAddr = addr_str.parse().map_err(|_| "Invalid address")?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    let ca_clone = (*ca).clone_shim();
    
    tauri::async_runtime::spawn(async move {
        tokio::select! {
            _ = proxy::start_proxy(app_handle, ca_clone, intercept_state, addr) => {},
            _ = rx => {
                println!("Stopping proxy listener on {}", addr);
            }
        }
    });
    
    Ok(tx)
}

#[tauri::command]
async fn toggle_proxy(app_handle: AppHandle, state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    let mut manager = state.proxy_manager.lock().await;
    manager.config.enabled = enabled;
    
    if !enabled {
        for (_, tx) in manager.active_listeners.drain() {
            let _ = tx.send(());
        }
    } else {
        let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
        let ca = Arc::new(ca::get_ca(app_data_dir.join("ca")));
        let bindings = manager.config.bindings.clone();
        
        for addr_str in bindings {
            if !manager.active_listeners.contains_key(&addr_str) {
                let tx = spawn_proxy_listener(
                    app_handle.clone(),
                    addr_str.clone(),
                    Arc::clone(&ca),
                    state.intercept_state.clone()
                ).await?;
                manager.active_listeners.insert(addr_str, tx);
            }
        }
    }
    
    let db_path = get_db_path(&app_handle);
    let config_json = serde_json::to_string(&manager.config).map_err(|e| e.to_string())?;
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO app_state (key, value) VALUES ('proxy_config', ?)", [config_json]).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
async fn update_network_settings(app_handle: AppHandle, state: State<'_, AppState>, bindings: Vec<String>) -> Result<(), String> {
    let mut manager = state.proxy_manager.lock().await;
    manager.config.bindings = bindings;
    
    if manager.config.enabled {
        for (_, tx) in manager.active_listeners.drain() {
            let _ = tx.send(());
        }
        
        let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
        let ca = Arc::new(ca::get_ca(app_data_dir.join("ca")));
        let bindings_to_start = manager.config.bindings.clone();
        
        for addr_str in bindings_to_start {
            let tx = spawn_proxy_listener(
                app_handle.clone(),
                addr_str.clone(),
                Arc::clone(&ca),
                state.intercept_state.clone()
            ).await?;
            manager.active_listeners.insert(addr_str, tx);
        }
    }
    
    let db_path = get_db_path(&app_handle);
    let config_json = serde_json::to_string(&manager.config).map_err(|e| e.to_string())?;
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO app_state (key, value) VALUES ('proxy_config', ?)", [config_json]).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
async fn update_variable(app_handle: AppHandle, id: String, updates: GlobalVariable) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    conn.execute(
        "UPDATE variables SET name = ?, active_index = ?, order_index = ? WHERE id = ?",
        rusqlite::params![updates.name, updates.active_index, updates.order_index, id],
    ).map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM variable_values WHERE variable_id = ?", [&id]).map_err(|e| e.to_string())?;
    for val in updates.values {
        conn.execute(
            "INSERT INTO variable_values (id, variable_id, name, value) VALUES (?, ?, ?, ?)",
            rusqlite::params![val.id, id, val.name, val.value],
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
async fn create_variable(app_handle: AppHandle, variable: GlobalVariable) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    conn.execute(
        "INSERT INTO variables (id, environment_id, name, active_index, order_index) VALUES (?, ?, ?, ?, ?)",
        rusqlite::params![variable.id, variable.environment_id, variable.name, variable.active_index, variable.order_index],
    ).map_err(|e| e.to_string())?;

    for val in variable.values {
        conn.execute(
            "INSERT INTO variable_values (id, variable_id, name, value) VALUES (?, ?, ?, ?)",
            rusqlite::params![val.id, variable.id, val.name, val.value],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn delete_variable(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM variables WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn create_environment(app_handle: AppHandle, id: String, name: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("INSERT INTO environments (id, name, is_active) VALUES (?, ?, 0)", [id, name]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn delete_environment(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM environments WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn set_active_environment(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("UPDATE environments SET is_active = 0", []).map_err(|e| e.to_string())?;
    conn.execute("UPDATE environments SET is_active = 1 WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn save_replacements_bulk(app_handle: AppHandle, replacements: Vec<Replacement>) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    for r in replacements {
        conn.execute(
            "INSERT OR REPLACE INTO replacements (id, type, pattern, replacement, description, is_active, order_index, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, strftime('%s', 'now'))",
            rusqlite::params![r.id, r.r_type, r.pattern, r.replacement, r.description, r.is_active, r.order_index],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn delete_replacement(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM replacements WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn update_replacement_order(app_handle: AppHandle, items: Vec<Replacement>) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    for r in items {
        conn.execute("UPDATE replacements SET order_index = ?, updated_at = strftime('%s', 'now') WHERE id = ?", rusqlite::params![r.order_index, r.id]).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn update_filter_config(app_handle: AppHandle, state: State<'_, AppState>, config: proxy::FilterConfig) -> Result<(), String> {
    let mut intercept = state.intercept_state.lock().await;
    intercept.filter_config = config.clone();

    let db_path = get_db_path(&app_handle);
    let config_json = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO app_state (key, value) VALUES ('filter_config', ?)", [config_json]).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn update_state(config: proxy::InterceptConfig, state: State<'_, AppState>) -> Result<(), String> {
    let mut intercept = state.intercept_state.lock().await;
    intercept.config = config;
    Ok(())
}

#[tauri::command]
async fn resume_flow(id: String, action: proxy::ResumeAction, state: State<'_, AppState>) -> Result<(), String> {
    let mut intercept = state.intercept_state.lock().await;
    if let Some(tx) = intercept.pending.remove(&id) {
        let _ = tx.send(action);
        Ok(())
    } else {
        Err("Flow not found".to_string())
    }
}

#[tauri::command]
async fn update_prefs(app_handle: AppHandle, prefs: serde_json::Value) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let prefs_json = serde_json::to_string(&prefs).map_err(|e| e.to_string())?;
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO app_state (key, value) VALUES ('prefs', ?)", [prefs_json]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn update_ui_layout(app_handle: AppHandle, layout: serde_json::Value) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let layout_json = serde_json::to_string(&layout).map_err(|e| e.to_string())?;
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO app_state (key, value) VALUES ('ui_layout', ?)", [layout_json]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn upload_file(app_handle: AppHandle, name: String, content: Vec<u8>) -> Result<String, String> {
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
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn sync_data(app_handle: AppHandle) -> Result<SyncData, String> {
    let db_path = get_db_path(&app_handle);
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

    // 2. Repeater Groups
    let mut stmt = conn.prepare("SELECT id, name, order_index FROM repeater_groups ORDER BY order_index").map_err(|e| e.to_string())?;
    let repeater_groups = stmt.query_map([], |row| {
        Ok(RepeaterGroup { id: row.get(0)?, name: row.get(1)?, order_index: row.get(2)? })
    }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    // 3. Repeater Requests
    let mut stmt = conn.prepare("SELECT id, name, group_id, method, url, headers, body, response_status, response_headers, response_body, hit_count FROM repeater_requests ORDER BY order_index").map_err(|e| e.to_string())?;
    let repeater_requests = stmt.query_map([], |row| {
        let headers: Vec<(String, String)> = serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default();
        let res_status: Option<u16> = row.get(7).ok();
        let response = if let Some(status) = res_status {
             let res_headers: Vec<(String, String)> = serde_json::from_str(&row.get::<_, String>(8)?).unwrap_or_default();
             Some(proxy::Traffic {
                 id: row.get::<_, String>(0)? + "_res", method: row.get(3)?, url: row.get(4)?, host: String::new(), status_code: status,
                 request_headers: headers.clone(), response_headers: res_headers,
                 request_body: row.get(6)?, response_body: row.get(9)?, phase: "response".to_string(),
                 is_intercepted: false, intercepted_at: None,
             })
        } else { None };

        Ok(RepeaterRequest {
            id: row.get(0)?, name: row.get(1)?, group_id: row.get(2)?, method: row.get(3)?, url: row.get(4)?,
            headers, body: row.get(6)?, response, hit_count: row.get(10)?,
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

    Ok(SyncData { history, repeater_groups, repeater_requests, environments, variables, replacements, prefs, ui_layout, toolkit_json, history_limits })
}

#[tauri::command]
async fn get_root_ca_pem(app_handle: AppHandle) -> Result<String, String> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let ca_dir = app_data_dir.join("ca");
    let ca = ca::get_ca(ca_dir);
    Ok(ca.cert_pem)
}

#[tauri::command]
async fn regenerate_root_ca(app_handle: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let ca_dir = app_data_dir.join("ca");

    // Stop all proxies
    let mut manager = state.proxy_manager.lock().await;
    for (_, tx) in manager.active_listeners.drain() {
        let _ = tx.send(());
    }

    // Delete and regenerate
    ca::delete_ca(ca_dir.clone());
    let new_ca = Arc::new(ca::get_ca(ca_dir.clone()));
    let cert_pem = new_ca.cert_pem.clone();

    // Restart if enabled
    if manager.config.enabled {
        let bindings = manager.config.bindings.clone();
        for addr_str in bindings {
            if let Ok(tx) = spawn_proxy_listener(
                app_handle.clone(),
                addr_str.clone(),
                Arc::clone(&new_ca),
                state.intercept_state.clone()
            ).await {
                manager.active_listeners.insert(addr_str, tx);
            }
        }
    }

    Ok(cert_pem)
}

#[tauri::command]
async fn get_proxy_status(state: State<'_, AppState>) -> Result<ProxyConfig, String> {
    let manager = state.proxy_manager.lock().await;
    Ok(manager.config.clone())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let migrations = db::get_migrations();

    let proxy_manager = Arc::new(Mutex::new(ProxyManager {
        active_listeners: HashMap::new(),
        config: ProxyConfig {
            bindings: vec!["8080".to_string()],
            enabled: true,
        }
    }));

    let intercept_state = Arc::new(Mutex::new(proxy::InterceptState {
        config: proxy::InterceptConfig::default(),
        filter_config: proxy::FilterConfig::default(),
        pending: std::collections::HashMap::new(),
    }));

    let state = AppState { 
        proxy_manager: Arc::clone(&proxy_manager),
        intercept_state: Arc::clone(&intercept_state),
    };

    tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_sql::Builder::default()
            .add_migrations("sqlite:mitm.db", migrations)
            .build())
        .invoke_handler(tauri::generate_handler![
            greet, 
            sync_data,
            get_root_ca_pem, 
            regenerate_root_ca,
            update_state,
            update_filter_config,
            resume_flow,
            repeater::create_repeater_item,
            repeater::update_repeater_request,
            repeater::delete_repeater_request,
            repeater::reorder_repeater_requests,
            repeater::create_repeater_group,
            repeater::delete_repeater_group,
            repeater::reorder_repeater_groups,
            repeater::rename_repeater_group,
            repeater::manage_group_assignment,
            repeater::execute_repeater_request,
            repeater::get_repeater_history,
            repeater::clear_repeater_history,
            repeater::delete_repeater_history_item,
            create_variable,
            update_variable,
            delete_variable,
            create_environment,
            delete_environment,
            set_active_environment,
            save_replacements_bulk,
            delete_replacement,
            update_replacement_order,
            update_prefs,
            update_ui_layout,
            save_state,
            upload_file,
            get_proxy_status,
            toggle_proxy,
            update_network_settings
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            // Ensure DB is initialized
            if let Err(e) = init_database(&app_handle) {
                eprintln!("Failed to initialize database: {}", e);
            }

            let state = app.state::<AppState>();
            let proxy_manager = Arc::clone(&state.proxy_manager);
            let intercept_state = Arc::clone(&state.intercept_state);
            
            // Get app data directory for CA storage
            let app_data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            let ca_dir = app_data_dir.join("ca");
            let db_path = app_data_dir.join("mitm.db");
            
            // Try to load config from DB
            tauri::async_runtime::block_on(async move {
                let mut manager = proxy_manager.lock().await;
                if let Ok(conn) = rusqlite::Connection::open(db_path.clone()) {
                    if let Ok(config_str) = conn.query_row::<String, _, _>(
                        "SELECT value FROM app_state WHERE key = 'proxy_config'",
                        [],
                        |row| row.get(0)
                    ) {
                        if let Ok(config) = serde_json::from_str::<ProxyConfig>(&config_str) {
                            manager.config = config;
                        }
                    }

                    let mut intercept = intercept_state.lock().await;
                    if let Ok(config_str) = conn.query_row::<String, _, _>(
                        "SELECT value FROM app_state WHERE key = 'filter_config'",
                        [],
                        |row| row.get(0)
                    ) {
                        if let Ok(config) = serde_json::from_str::<proxy::FilterConfig>(&config_str) {
                            intercept.filter_config = config;
                        }
                    }
                }

                if manager.config.enabled {
                    let ca = Arc::new(ca::get_ca(ca_dir));
                    let bindings = manager.config.bindings.clone();
                    for addr_str in bindings {
                        if let Ok(tx) = spawn_proxy_listener(
                            app_handle.clone(),
                            addr_str.clone(),
                            Arc::clone(&ca),
                            intercept_state.clone()
                        ).await {
                            manager.active_listeners.insert(addr_str, tx);
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
