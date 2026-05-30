mod ca;
mod proxy;
mod db;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_rustls::rustls;
use tauri::{AppHandle, Manager, State};

pub struct ProxyManager {
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
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
    
    // Stop proxy
    {
        let mut manager = state.proxy_manager.lock().await;
        if let Some(tx) = manager.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }

    // Delete and regenerate
    ca::delete_ca(ca_dir.clone());
    let ca = ca::get_ca(ca_dir.clone());
    let cert_pem = ca.cert_pem.clone();

    // Restart proxy
    let app_handle_clone = app_handle.clone();
    let proxy_manager = Arc::clone(&state.proxy_manager);
    let intercept_state = Arc::clone(&state.intercept_state);
    
    tauri::async_runtime::spawn(async move {
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let mut manager = proxy_manager.lock().await;
            manager.shutdown_tx = Some(tx);
        }

        let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
        let proxy_task = proxy::start_proxy(app_handle_clone, ca, intercept_state, addr);
        
        tokio::select! {
            _ = proxy_task => {
                eprintln!("Proxy task finished unexpectedly");
            }
            _ = rx => {
                println!("Proxy shutting down for CA regeneration");
            }
        }
    });

    Ok(cert_pem)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let migrations = db::get_migrations();

    let proxy_manager = Arc::new(Mutex::new(ProxyManager { shutdown_tx: None }));
    let intercept_state = Arc::new(Mutex::new(proxy::InterceptState {
        config: proxy::InterceptConfig::default(),
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
            resume_flow,
            create_repeater_item
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            let state = app.state::<AppState>();
            let proxy_manager = Arc::clone(&state.proxy_manager);
            let intercept_state = Arc::clone(&state.intercept_state);
            
            // Get app data directory for CA storage
            let app_data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            let ca_dir = app_data_dir.join("ca");
            
            // Initialize CA
            let ca = ca::get_ca(ca_dir);
            
            // Spawn proxy server in a background tokio task
            tauri::async_runtime::spawn(async move {
                let (tx, rx) = tokio::sync::oneshot::channel();
                {
                    let mut manager = proxy_manager.lock().await;
                    manager.shutdown_tx = Some(tx);
                }

                let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
                
                let proxy_task = proxy::start_proxy(app_handle, ca, intercept_state, addr);
                
                tokio::select! {
                    _ = proxy_task => {
                        eprintln!("Proxy task finished unexpectedly");
                    }
                    _ = rx => {
                        println!("Proxy shutting down");
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
