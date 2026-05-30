mod ca;
mod proxy;
mod db;

use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio_rustls::rustls;
use tauri::{AppHandle, Manager, State};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ProxyConfig {
    pub bindings: Vec<String>,
    pub enabled: bool,
}

pub struct ProxyManager {
    pub active_listeners: HashMap<String, tokio::sync::oneshot::Sender<()>>,
    pub config: ProxyConfig,
}

#[derive(serde::Serialize)]
pub struct RepeaterGroup {
    pub id: String,
    pub name: String,
    pub order_index: i32,
}

#[derive(serde::Serialize)]
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

#[derive(serde::Serialize)]
pub struct VariableValue {
    pub id: String,
    pub name: String,
    pub value: String,
}

#[derive(serde::Serialize)]
pub struct GlobalVariable {
    pub id: String,
    pub environment_id: String,
    pub name: String,
    pub active_index: i32,
    pub order_index: i32,
    pub values: Vec<VariableValue>,
}

#[derive(serde::Serialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub is_active: bool,
}

#[derive(serde::Serialize)]
pub struct Replacement {
    pub id: String,
    pub r_type: String, // Field name in DB is 'type', but we need to map it carefully
    pub pattern: String,
    pub replacement: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub order_index: i32,
}

#[derive(serde::Serialize)]
pub struct SyncData {
    pub history: Vec<proxy::Traffic>,
    pub repeater_groups: Vec<RepeaterGroup>,
    pub repeater_requests: Vec<RepeaterRequest>,
    pub environments: Vec<Environment>,
    pub variables: Vec<GlobalVariable>,
    pub replacements: Vec<Replacement>,
}

pub struct AppState {
    pub proxy_manager: Arc<Mutex<ProxyManager>>,
    pub intercept_state: Arc<Mutex<proxy::InterceptState>>,
}

#[derive(serde::Deserialize)]
pub struct RepeaterResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[derive(serde::Deserialize)]
pub struct CreateRepeaterItem {
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub response: Option<RepeaterResponse>,
}

fn get_db_path(app_handle: &AppHandle) -> std::path::PathBuf {
    app_handle.path().app_data_dir().expect("Failed to get app data dir").join("mitm.db")
}

async fn spawn_proxy_listener(
    app_handle: AppHandle,
    addr_str: String,
    ca: Arc<ca::CA>,
    intercept_state: Arc<Mutex<proxy::InterceptState>>,
) -> Result<tokio::sync::oneshot::Sender<()>, String> {
    let addr: SocketAddr = if addr_str.contains(':') {
        addr_str.parse().map_err(|e| format!("Invalid address {}: {}", addr_str, e))?
    } else {
        format!("0.0.0.0:{}", addr_str).parse().map_err(|e| format!("Invalid port {}: {}", addr_str, e))?
    };

    let (tx, rx) = tokio::sync::oneshot::channel();
    let app_handle_clone = app_handle.clone();
    
    tauri::async_runtime::spawn(async move {
        let proxy_task = proxy::start_proxy(app_handle_clone, (*ca).clone_shim(), intercept_state, addr);
        
        tokio::select! {
            res = proxy_task => {
                if let Err(e) = res {
                    eprintln!("Proxy on {} failed: {}", addr, e);
                }
            }
            _ = rx => {
                println!("Proxy on {} shutting down", addr);
            }
        }
    });

    Ok(tx)
}

#[tauri::command]
async fn get_proxy_status(state: State<'_, AppState>) -> Result<ProxyConfig, String> {
    let manager = state.proxy_manager.lock().await;
    Ok(manager.config.clone())
}

#[tauri::command]
async fn toggle_proxy(app_handle: AppHandle, state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    let mut manager = state.proxy_manager.lock().await;
    manager.config.enabled = enabled;
    
    // Stop all if disabling
    if !enabled {
        for (_, tx) in manager.active_listeners.drain() {
            let _ = tx.send(());
        }
    } else {
        // Start all configured bindings
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
    
    // Save to DB
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
    
    // If enabled, restart all listeners
    if manager.config.enabled {
        // Stop current
        for (_, tx) in manager.active_listeners.drain() {
            let _ = tx.send(());
        }
        
        // Start new
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
    
    // Save to DB
    let db_path = get_db_path(&app_handle);
    let config_json = serde_json::to_string(&manager.config).map_err(|e| e.to_string())?;
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO app_state (key, value) VALUES ('proxy_config', ?)", [config_json]).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
async fn get_state(app_handle: AppHandle) -> Result<serde_json::Value, String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT key, value FROM app_state").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }).map_err(|e| e.to_string())?;
    
    let mut map = serde_json::Map::new();
    for row in rows {
        if let Ok((key, value)) = row {
            if let Ok(json_val) = serde_json::from_str(&value) {
                map.insert(key, json_val);
            } else {
                map.insert(key, serde_json::Value::String(value));
            }
        }
    }
    
    Ok(serde_json::Value::Object(map))
}

#[tauri::command]
async fn update_filter_config(app_handle: AppHandle, state: State<'_, AppState>, config: proxy::FilterConfig) -> Result<(), String> {
    let mut intercept = state.intercept_state.lock().await;
    intercept.filter_config = config.clone();

    // Save to DB
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
async fn create_repeater_item(app_handle: AppHandle, item: CreateRepeaterItem) -> Result<String, String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let id = uuid::Uuid::new_v4().to_string();
    let headers_json = serde_json::to_string(&item.headers).unwrap_or_default();
    
    let (res_status, res_headers, res_body) = if let Some(res) = &item.response {
        (Some(res.status), Some(serde_json::to_string(&res.headers).unwrap_or_default()), Some(res.body.clone()))
    } else {
        (None, None, None)
    };

    conn.execute(
        "INSERT INTO repeater_requests (id, name, method, url, headers, body, response_status, response_headers, response_body) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![id, item.name, item.method, item.url, headers_json, item.body, res_status, res_headers, res_body],
    ).map_err(|e| e.to_string())?;

    Ok(id)
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

    Ok(SyncData { history, repeater_groups, repeater_requests, environments, variables, replacements })
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
            let tx = spawn_proxy_listener(
                app_handle.clone(),
                addr_str.clone(),
                Arc::clone(&new_ca),
                state.intercept_state.clone()
            ).await.map_err(|e| e.to_string())?;
            manager.active_listeners.insert(addr_str, tx);
        }
    }

    Ok(cert_pem)
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
            create_repeater_item,
            get_proxy_status,
            toggle_proxy,
            update_network_settings,
            get_state
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
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
