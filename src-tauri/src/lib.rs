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

pub struct AppState {
    pub proxy_manager: Arc<Mutex<ProxyManager>>,
    pub intercept_state: Arc<Mutex<proxy::InterceptState>>,
}

#[derive(serde::Deserialize)]
pub struct RepeaterResponse {
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
}

#[derive(serde::Deserialize)]
pub struct CreateRepeaterItem {
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: std::collections::HashMap<String, String>,
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
async fn create_repeater_item(item: CreateRepeaterItem) -> Result<String, String> {
    println!("Staging to Repeater: {} {}", item.method, item.url);
    // TODO: Actually insert into DB. For now, just return a fake ID.
    Ok(uuid::Uuid::new_v4().to_string())
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn get_history() -> Vec<proxy::Traffic> {
    Vec::new()
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
            get_history, 
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
