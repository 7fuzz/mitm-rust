pub mod ca;
pub mod proxy;
pub mod db;
pub mod repeater;
pub mod repeater_execute;
pub mod models;
pub mod workspace;
pub mod state;
pub mod history;
pub mod db_viewer;
pub mod websocket;
pub mod webhook;

use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tauri::{AppHandle, Manager, State, Emitter};
use crate::models::{ProxyConfig, ProxyManager, AppState};
use serde::{Serialize, Deserialize};

async fn spawn_proxy_listener(
    app_handle: AppHandle,
    addr_str: String,
    ca: Arc<ca::CA>,
    intercept_state: Arc<Mutex<proxy::InterceptState>>,
) -> Result<tokio::sync::oneshot::Sender<()>, String> {
    // Support "8080" as "0.0.0.0:8080"
    let full_addr = if !addr_str.contains(':') {
        format!("0.0.0.0:{}", addr_str)
    } else {
        addr_str.clone()
    };

    let addr: SocketAddr = full_addr.parse().map_err(|_| format!("Invalid address: {}", addr_str))?;
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
async fn set_proxy_mode(app_handle: AppHandle, state: State<'_, AppState>, mode: String) -> Result<(), String> {
    let mut manager = state.proxy_manager.lock().await;
    let old_mode = manager.config.proxy_mode.clone();
    manager.config.proxy_mode = mode.clone();
    
    let is_now_enabled = mode != "off";
    let was_enabled = old_mode != "off";
    
    if !is_now_enabled {
        for (_, tx) in manager.active_listeners.drain() {
            let _ = tx.send(());
        }
    } else if !was_enabled {
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
    
    let db_path = db::get_db_path(&app_handle);
    let config_json = serde_json::to_string(&manager.config).map_err(|e| e.to_string())?;
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO app_state (key, value) VALUES ('proxy_config', ?)", [config_json]).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
async fn update_network_settings(app_handle: AppHandle, state: State<'_, AppState>, bindings: Vec<String>) -> Result<(), String> {
    let mut manager = state.proxy_manager.lock().await;
    manager.config.bindings = bindings;
    
    if manager.config.proxy_mode != "off" {
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
    
    let db_path = db::get_db_path(&app_handle);
    let config_json = serde_json::to_string(&manager.config).map_err(|e| e.to_string())?;
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO app_state (key, value) VALUES ('proxy_config', ?)", [config_json]).map_err(|e| e.to_string())?;
    
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
async fn update_filter_config(app_handle: AppHandle, state: State<'_, AppState>, config: proxy::FilterConfig) -> Result<(), String> {
    let mut intercept = state.intercept_state.lock().await;
    intercept.filter_config = config.clone();

    let db_path = db::get_db_path(&app_handle);
    let config_json = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO app_state (key, value) VALUES ('filter_config', ?)", [config_json]).map_err(|e| e.to_string())?;

    Ok(())
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

    let mut manager = state.proxy_manager.lock().await;
    for (_, tx) in manager.active_listeners.drain() {
        let _ = tx.send(());
    }

    ca::delete_ca(ca_dir.clone());
    let new_ca = Arc::new(ca::get_ca(ca_dir.clone()));
    let cert_pem = new_ca.cert_pem.clone();

    if manager.config.proxy_mode != "off" {
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyStatusResponse {
    pub mode: String,
    pub bindings: Vec<String>,
}

#[tauri::command]
async fn get_proxy_status(state: State<'_, AppState>) -> Result<ProxyStatusResponse, String> {
    let manager = state.proxy_manager.lock().await;
    Ok(ProxyStatusResponse {
        mode: manager.config.proxy_mode.clone(),
        bindings: manager.config.bindings.clone(),
    })
}

#[tauri::command]
async fn get_websocket_messages(app_handle: AppHandle, connection_id: String) -> Result<Vec<crate::models::WsMessage>, String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let mut stmt = conn.prepare("SELECT id, connection_id, direction, msg_type, payload, timestamp, is_intercepted FROM websocket_messages WHERE connection_id = ? ORDER BY timestamp ASC").map_err(|e| e.to_string())?;
    let ws_messages = stmt.query_map([connection_id], |row| {
        let is_intercepted_val: i32 = row.get(6)?;
        Ok(crate::models::WsMessage {
            id: row.get(0)?,
            connection_id: row.get(1)?,
            direction: row.get(2)?,
            msg_type: row.get(3)?,
            payload: row.get(4)?,
            timestamp: row.get(5)?,
            is_intercepted: is_intercepted_val != 0,
        })
    }).map_err(|e| e.to_string())?;
    
    let mut list = Vec::new();
    for msg in ws_messages {
        if let Ok(m) = msg {
            list.push(m);
        }
    }
    Ok(list)
}

#[tauri::command]
async fn resume_ws_flow(id: String, action: crate::models::WsResumeAction, state: State<'_, AppState>) -> Result<(), String> {
    let mut pending = state.pending_ws.lock().await;
    if let Some(tx) = pending.remove(&id) {
        let _ = tx.send(action);
        Ok(())
    } else {
        Err("WebSocket frame flow not found".to_string())
    }
}

#[tauri::command]
async fn send_websocket_message(
    app_handle: AppHandle,
    connection_id: String,
    direction: String,
    payload: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    let active_ws = state.active_websockets.lock().await;
    if let Some(conn) = active_ws.get(&connection_id) {
        let msg = tokio_tungstenite::tungstenite::Message::Text(payload.clone());
        if direction == "to_server" {
            conn.to_server_tx.send(msg).map_err(|e| e.to_string())?;
        } else if direction == "to_client" {
            conn.to_client_tx.send(msg).map_err(|e| e.to_string())?;
        } else {
            return Err("Invalid direction".to_string());
        }

        // Log the injected message to the database
        let message_id = uuid::Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        
        let db_path = db::get_db_path(&app_handle);
        let conn_db = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
        conn_db.execute(
            "INSERT INTO websocket_messages (id, connection_id, direction, msg_type, payload, timestamp, is_intercepted) VALUES (?, ?, ?, ?, ?, ?, 0)",
            rusqlite::params![
                message_id,
                connection_id,
                direction,
                "text",
                payload,
                now as i64
            ]
        ).map_err(|e| e.to_string())?;

        // Emit message to frontend so it shows up in real time
        let ws_message = crate::models::WsMessage {
            id: message_id,
            connection_id: connection_id.clone(),
            direction: direction.clone(),
            msg_type: "text".to_string(),
            payload: payload.clone(),
            timestamp: now,
            is_intercepted: false,
        };
        let _ = app_handle.emit("ws_message_captured", &ws_message);

        Ok(())
    } else {
        Err("Active WebSocket connection not found".to_string())
    }
}

#[tauri::command]
async fn get_webhook_endpoints(app_handle: AppHandle) -> Result<Vec<models::WebhookEndpoint>, String> {
    webhook::get_endpoints_db(&app_handle)
}

#[tauri::command]
async fn create_webhook_endpoint(app_handle: AppHandle, endpoint: models::WebhookEndpoint) -> Result<models::WebhookEndpoint, String> {
    webhook::create_endpoint_db(&app_handle, endpoint)
}

#[tauri::command]
async fn update_webhook_endpoint(app_handle: AppHandle, endpoint: models::WebhookEndpoint) -> Result<(), String> {
    webhook::update_endpoint_db(&app_handle, endpoint)
}

#[tauri::command]
async fn delete_webhook_endpoint(app_handle: AppHandle, id: String) -> Result<(), String> {
    webhook::delete_endpoint_db(&app_handle, id)
}

#[tauri::command]
async fn get_webhook_deliveries(app_handle: AppHandle, limit: Option<i32>) -> Result<Vec<models::WebhookDelivery>, String> {
    webhook::get_deliveries_db(&app_handle, limit.unwrap_or(100))
}

#[tauri::command]
async fn clear_webhook_deliveries(app_handle: AppHandle) -> Result<(), String> {
    webhook::clear_deliveries_db(&app_handle)
}

#[tauri::command]
async fn delete_webhook_delivery(app_handle: AppHandle, id: String) -> Result<(), String> {
    webhook::delete_delivery_db(&app_handle, id)
}

#[tauri::command]
async fn forward_webhook_delivery(app_handle: AppHandle, delivery_id: String, target_url: String) -> Result<models::WebhookForwardResult, String> {
    webhook::forward_delivery_http(&app_handle, delivery_id, target_url).await
}

#[tauri::command]
async fn trigger_webhook_request(req: models::WebhookTriggerRequest) -> Result<models::WebhookForwardResult, String> {
    webhook::trigger_webhook_request(req).await
}

#[tauri::command]
async fn get_webhook_listener_status(state: State<'_, AppState>) -> Result<models::WebhookListenerConfig, String> {
    let manager = state.webhook_manager.lock().await;
    Ok(manager.config.clone())
}

#[tauri::command]
async fn start_webhook_listener(app_handle: AppHandle, state: State<'_, AppState>, port: u16) -> Result<(), String> {
    let mut manager = state.webhook_manager.lock().await;
    if let Some(tx) = manager.active_listener.take() {
        let _ = tx.send(());
    }
    let tx = webhook::start_webhook_server(app_handle, port).await?;
    manager.active_listener = Some(tx);
    manager.config.port = port;
    manager.config.is_running = true;
    Ok(())
}

#[tauri::command]
async fn stop_webhook_listener(state: State<'_, AppState>) -> Result<(), String> {
    let mut manager = state.webhook_manager.lock().await;
    if let Some(tx) = manager.active_listener.take() {
        let _ = tx.send(());
    }
    manager.config.is_running = false;
    Ok(())
}

#[derive(Serialize)]
pub struct SignatureResult {
    pub header_name: String,
    pub header_value: String,
}

#[tauri::command]
fn calculate_webhook_signature(secret: String, body: String, provider: String) -> SignatureResult {
    let (name, val) = webhook::calculate_hmac_signature(&secret, &body, &provider);
    SignatureResult {
        header_name: name,
        header_value: val,
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let proxy_manager = Arc::new(Mutex::new(ProxyManager {
        active_listeners: HashMap::new(),
        config: ProxyConfig {
            bindings: vec!["8080".to_string()],
            proxy_mode: "normal".to_string(),
        }
    }));

    let intercept_state = Arc::new(Mutex::new(proxy::InterceptState {
        config: proxy::InterceptConfig::default(),
        filter_config: proxy::FilterConfig::default(),
        pending: std::collections::HashMap::new(),
    }));

    let active_websockets = Arc::new(Mutex::new(HashMap::new()));
    let pending_ws = Arc::new(Mutex::new(HashMap::new()));

    let webhook_manager = Arc::new(Mutex::new(models::WebhookManager {
        active_listener: None,
        config: models::WebhookListenerConfig {
            port: 9000,
            is_running: false,
        },
    }));

    let state = AppState { 
        proxy_manager: Arc::clone(&proxy_manager),
        intercept_state: Arc::clone(&intercept_state),
        active_websockets,
        pending_ws,
        webhook_manager: Arc::clone(&webhook_manager),
    };

    tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            greet, 
            state::sync_data,
            state::get_repeater_requests,
            get_root_ca_pem, 
            regenerate_root_ca,
            update_state,
            update_filter_config,
            resume_flow,
            history::clear_history,
            history::delete_history_item,
            history::save_traffic_history,
            repeater::create_repeater_item,
            repeater::update_repeater_request,
            repeater::delete_repeater_request,
            repeater::bulk_delete_repeater_groups,
            repeater::reorder_repeater_requests,
            repeater::create_repeater_group,
            repeater::delete_repeater_group,
            repeater::reorder_repeater_groups,
            repeater::rename_repeater_group,
            repeater::update_repeater_group_description,
            repeater::update_repeater_group_extractions,
            repeater::manage_group_assignment,
            repeater_execute::execute_repeater_request,
            repeater::get_repeater_history,
            repeater::clear_repeater_history,
            repeater::delete_repeater_history_item,
            repeater::import_repeater_data,
            workspace::create_variable,
            workspace::update_variable,
            workspace::delete_variable,
            workspace::create_environment,
            workspace::delete_environment,
            workspace::set_active_environment,
            workspace::save_replacements_bulk,
            workspace::delete_replacement,
            workspace::update_replacement_order,
            state::update_prefs,
            state::update_ui_layout,
            state::save_state,
            state::purge_all_data,
            state::purge_selective_data,
            state::upload_file,
            state::upload_file_base64,
            db_viewer::get_database_tables,
            db_viewer::get_table_data,
            get_proxy_status,
            set_proxy_mode,
            update_network_settings,
            get_websocket_messages,
            resume_ws_flow,
            send_websocket_message,
            get_webhook_endpoints,
            create_webhook_endpoint,
            update_webhook_endpoint,
            delete_webhook_endpoint,
            get_webhook_deliveries,
            clear_webhook_deliveries,
            delete_webhook_delivery,
            forward_webhook_delivery,
            trigger_webhook_request,
            get_webhook_listener_status,
            start_webhook_listener,
            stop_webhook_listener,
            calculate_webhook_signature
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            if let Err(e) = db::init_database(&app_handle) {
                eprintln!("Failed to initialize database: {}", e);
            }

            // Automatically start Webhook listener on saved port (from prefs), default 9000
            let app_handle_for_wh = app_handle.clone();
            let state_wh = app_handle.state::<AppState>();
            let wh_mgr = Arc::clone(&state_wh.webhook_manager);
            {
                let db_path_wh = app_handle.path().app_data_dir()
                    .expect("Failed to get app data dir")
                    .join("mitm.db");
                let wh_port: u16 = rusqlite::Connection::open(&db_path_wh)
                    .ok()
                    .and_then(|conn| {
                        conn.query_row::<String, _, _>(
                            "SELECT value FROM app_state WHERE key = 'prefs'",
                            [],
                            |row| row.get(0),
                        ).ok()
                    })
                    .and_then(|prefs_str| serde_json::from_str::<serde_json::Value>(&prefs_str).ok())
                    .and_then(|v| v.get("webhookPort").and_then(|p| p.as_u64()))
                    .map(|p| p as u16)
                    .unwrap_or(9000);

                tauri::async_runtime::spawn(async move {
                    if let Ok(tx) = webhook::start_webhook_server(app_handle_for_wh, wh_port).await {
                        let mut mgr = wh_mgr.lock().await;
                        mgr.active_listener = Some(tx);
                        mgr.config.port = wh_port;
                        mgr.config.is_running = true;
                    }
                });
            }

            let state = app.state::<AppState>();
            let proxy_manager = Arc::clone(&state.proxy_manager);
            let intercept_state = Arc::clone(&state.intercept_state);
            
            let app_data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            let ca_dir = app_data_dir.join("ca");
            let db_path = app_data_dir.join("mitm.db");
            
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

                if manager.config.proxy_mode != "off" {
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
