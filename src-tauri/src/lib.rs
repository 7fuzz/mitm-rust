pub mod ca;
pub mod commands;
pub mod db;
pub mod proxy;
pub mod state;

use tokio::sync::mpsc;
use tauri::Manager;
use state::{AppState, HistoryEntry};

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
            app.manage(app_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::history_cmd::get_history_logs,
            commands::history_cmd::get_history_detail,
            commands::history_cmd::clear_history_logs,
            commands::proxy_cmd::get_proxy_state,
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
