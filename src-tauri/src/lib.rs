pub mod ca;
pub mod commands;
pub mod db;
pub mod proxy;
pub mod state;

use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tauri::Manager;
use state::{AppState, HistoryEntry};
use ca::RootCa;
use proxy::start_proxy_server;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let (history_tx, history_rx) = mpsc::channel::<HistoryEntry>(2000);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .setup(move |app| {
            let app_handle = app.handle().clone();

            let db_path = db::init_database(&app_handle)
                .expect("Failed to initialize SQLite database");

            db::actor::start_history_actor(db_path.clone(), history_rx);

            let app_state = AppState::new(db_path, history_tx);

            // Auto-start proxy server if initial mode is ON (default)
            if app_state.is_proxy_active() {
                let app_handle_clone = app_handle.clone();
                let app_data_dir = app_handle.path().app_data_dir().expect("Failed to get app data dir");
                let ca_dir = app_data_dir.join("ca");
                let ca = Arc::new(RootCa::load_or_generate(ca_dir).expect("Failed to generate Root CA"));

                let (stop_tx, stop_rx) = oneshot::channel::<()>();
                
                let state_arc = Arc::new(AppState {
                    db_path: app_state.db_path.clone(),
                    history_tx: app_state.history_tx.clone(),
                    proxy_active: std::sync::atomic::AtomicBool::new(true),
                    broadcast_tx: app_state.broadcast_tx.clone(),
                    proxy_config: Arc::clone(&app_state.proxy_config),
                    stop_signal: Arc::new(tokio::sync::Mutex::new(Some(stop_tx))),
                    pending_flows: Arc::clone(&app_state.pending_flows),
                    rules: Arc::clone(&app_state.rules),
                });

                tauri::async_runtime::spawn(async move {
                    let _ = start_proxy_server(app_handle_clone, state_arc, ca, "127.0.0.1:8080".to_string(), stop_rx).await;
                });
            }

            app.manage(app_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::history_cmd::get_history_logs,
            commands::history_cmd::get_history_detail,
            commands::history_cmd::clear_history_logs,
            commands::proxy_cmd::get_proxy_state,
            commands::proxy_cmd::set_proxy_mode,
            commands::proxy_cmd::toggle_proxy,
            commands::proxy_cmd::start_proxy,
            commands::proxy_cmd::stop_proxy,
            commands::proxy_cmd::toggle_proxy_legacy,
            commands::proxy_cmd::get_proxy_status,
            commands::intercept_cmd::toggle_interceptor,
            commands::intercept_cmd::update_intercept_rules,
            commands::intercept_cmd::get_intercept_rules,
            commands::intercept_cmd::forward_intercepted_flow,
            commands::intercept_cmd::drop_intercepted_flow,
            commands::intercept_cmd::forward_all_intercepted_flows,
            commands::intercept_cmd::drop_all_intercepted_flows,
            commands::intercept_cmd::get_pending_flows,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
