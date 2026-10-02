pub mod ca;
pub mod collections;
pub mod commands;
pub mod db;
pub mod encoding;
pub mod proxy;
pub mod repeater;
pub mod state;
pub mod webhook;
pub mod workspace;
pub mod ws;

use tokio::sync::mpsc;
use tauri::Manager;
use state::{AppState, HistoryEntry};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let (history_tx, history_rx) = mpsc::channel::<HistoryEntry>(2000);
    let (rewrite_tx, rewrite_rx) = mpsc::channel::<state::RewriteHistoryEntry>(2000);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .setup(move |app| {
            let app_handle = app.handle().clone();

            // Perform DB initialization safely; if it fails, log error so window opens and recovery modal handles it!
            match db::init_database(&app_handle) {
                Ok(db_path) => {
                    db::actor::start_history_actor(db_path.clone(), history_rx);
                    db::actor::start_rewrite_history_actor(db_path.clone(), rewrite_rx);
                    let app_state = AppState::new(db_path, history_tx.clone(), rewrite_tx.clone());

                    // Auto-start all enabled listeners if initial mode is ON (default)
                    if app_state.is_proxy_active() {
                        let app_handle_listeners = app_handle.clone();
                        let state_for_listeners = app_state.clone();
                        tauri::async_runtime::spawn(async move {
                            commands::proxy_cmd::start_all_listeners(&app_handle_listeners, &state_for_listeners).await;
                        });
                    }
                    app.manage(app_state);
                }
                Err(err) => {
                    eprintln!("[DB Startup Warning] Database init deferred due to error: {}", err);
                    // Minimal app state fallback so commands can still be invoked
                    let fallback_path = db::get_db_path(&app_handle);
                    let app_state = AppState::new(fallback_path, history_tx, rewrite_tx);
                    app.manage(app_state);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ca_cmd::get_root_ca_pem,
            commands::ca_cmd::export_root_ca,
            commands::ca_cmd::regenerate_root_ca,

            commands::db_browser_cmd::list_db_tables,
            commands::db_browser_cmd::query_db_table,
            commands::db_browser_cmd::get_db_row,
            commands::db_browser_cmd::run_db_query,

            commands::db_recovery_cmd::run_database_migrations,
            commands::db_recovery_cmd::backup_and_reset_database,
            commands::db_recovery_cmd::export_database_file,
            commands::db_recovery_cmd::quit_application,

            commands::history_cmd::get_history_logs,
            commands::history_cmd::get_history_detail,
            commands::history_cmd::get_history_count,
            commands::history_cmd::clear_history_logs,
            commands::history_cmd::get_history_settings,
            commands::history_cmd::update_history_settings,

            commands::proxy_cmd::get_proxy_state,
            commands::proxy_cmd::set_proxy_mode,
            commands::proxy_cmd::toggle_proxy,
            commands::proxy_cmd::start_proxy,
            commands::proxy_cmd::stop_proxy,
            commands::proxy_cmd::toggle_proxy_legacy,
            commands::proxy_cmd::get_proxy_status,
            commands::proxy_cmd::update_network_settings,
            commands::proxy_cmd::set_ws_mitm_enabled,
            commands::proxy_cmd::get_listener_configs,
            commands::proxy_cmd::list_host_ips,
            commands::proxy_cmd::add_listener,
            commands::proxy_cmd::remove_listener,
            commands::proxy_cmd::update_listener,
            commands::proxy_cmd::set_listener_enabled,
            commands::intercept_cmd::toggle_interceptor,
            commands::intercept_cmd::update_intercept_rules,
            commands::intercept_cmd::set_intercept_source_scope,
            commands::intercept_cmd::get_intercept_rules,
            commands::intercept_cmd::forward_intercepted_flow,
            commands::intercept_cmd::drop_intercepted_flow,
            commands::intercept_cmd::forward_all_intercepted_flows,
            commands::intercept_cmd::drop_all_intercepted_flows,
            commands::intercept_cmd::get_pending_flows,
            commands::intercept_cmd::focus_app_window,
            commands::intercept_cmd::set_focus_preference,
            commands::intercept_cmd::get_focus_preference,

            commands::ui_prefs_cmd::get_ui_preferences,
            commands::ui_prefs_cmd::set_ui_preference,

            commands::rewrite_cmd::get_rewrite_rules,
            commands::rewrite_cmd::save_rewrite_rules,
            commands::rewrite_cmd::toggle_rewrite_enabled,
            commands::rewrite_cmd::get_rewrite_enabled,
            commands::rewrite_cmd::get_rewrite_source_scope,
            commands::rewrite_cmd::set_rewrite_source_scope,
            commands::rewrite_cmd::get_rewrite_history,
            commands::rewrite_cmd::clear_rewrite_history,

            commands::repeater_cmd::get_repeater_tabs,
            commands::repeater_cmd::create_repeater_tab,
            commands::repeater_cmd::update_repeater_tab,
            commands::repeater_cmd::delete_repeater_tab,
            commands::repeater_cmd::execute_repeater_request,
            commands::repeater_cmd::get_repeater_history,
            commands::repeater_cmd::insert_repeater_history,

            commands::workspace_cmd::get_workspaces,
            commands::workspace_cmd::create_workspace,
            commands::workspace_cmd::update_workspace,
            commands::workspace_cmd::delete_workspace,
            commands::workspace_cmd::set_active_workspace,
            commands::workspace_cmd::import_workspace_json,
            commands::workspace_cmd::get_workspace_environments,
            commands::workspace_cmd::save_workspace_environment,
            commands::workspace_cmd::delete_workspace_environment,
            commands::workspace_cmd::export_workspace_json,
            commands::workspace_cmd::export_workspace_file,

            commands::collection_cmd::get_collections,
            commands::collection_cmd::create_collection,
            commands::collection_cmd::update_collection,
            commands::collection_cmd::delete_collection,
            commands::collection_cmd::move_collection,
            commands::collection_cmd::duplicate_collection,
            commands::collection_cmd::create_request,
            commands::collection_cmd::update_request,
            commands::collection_cmd::delete_request,
            commands::collection_cmd::move_request,
            commands::collection_cmd::duplicate_request,
            commands::collection_cmd::execute_collection_request,
            commands::collection_cmd::get_request_histories,
            commands::collection_cmd::clear_request_histories,
            commands::collection_cmd::read_file_as_base64,
            commands::collection_cmd::preview_collection_request,

            commands::webhook_cmd::get_webhook_endpoints,
            commands::webhook_cmd::create_webhook_endpoint,
            commands::webhook_cmd::delete_webhook_endpoint,
            commands::webhook_cmd::get_webhook_deliveries,
            commands::webhook_cmd::clear_webhook_deliveries,
            commands::webhook_cmd::get_webhook_listener_status,
            commands::webhook_cmd::start_webhook_listener,
            commands::webhook_cmd::set_webhook_port,
            commands::webhook_cmd::stop_webhook_listener,
            commands::webhook_cmd::calculate_webhook_signature,
            commands::webhook_cmd::replay_webhook_delivery,

            commands::ws_cmd::get_ws_connections,
            commands::ws_cmd::get_ws_messages,
            commands::ws_cmd::connect_ws_client,
            commands::ws_cmd::disconnect_ws_client,
            commands::ws_cmd::send_ws_message,
            commands::ws_cmd::clear_ws_messages,
            commands::ws_cmd::delete_ws_connection,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
