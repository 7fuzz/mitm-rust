mod ca;
mod proxy;

use hudsucker::Proxy;
use std::net::SocketAddr;
use tauri::Manager;
use tauri_plugin_sql::{Migration, MigrationKind};

#[tauri::command]
async fn get_history() -> Vec<proxy::Traffic> {
    // Placeholder - will return from DB later
    Vec::new()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migrations = vec![
        Migration {
            version: 1,
            description: "create history table",
            sql: "CREATE TABLE IF NOT EXISTS history (
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
            );",
            kind: MigrationKind::Up,
        }
    ];

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_sql::Builder::default()
            .add_migrations("sqlite:mitm.db", migrations)
            .build())
        .invoke_handler(tauri::generate_handler![greet, get_history])
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            // Get app data directory for CA storage
            let app_data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            let ca_dir = app_data_dir.join("ca");
            
            // Initialize CA
            let ca = ca::get_ca(ca_dir);
            
            // Spawn proxy server in a background tokio task
            tauri::async_runtime::spawn(async move {
                let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
                let proxy = Proxy::builder()
                    .with_addr(addr)
                    .with_rustls_client()
                    .with_ca(ca)
                    .with_http_handler(proxy::MitmHandler { app_handle })
                    .build();

                println!("Proxy server listening on {}", addr);
                
                if let Err(e) = proxy.start(tauri::async_runtime::handle().clone()).await {
                    eprintln!("Proxy server error: {}", e);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
