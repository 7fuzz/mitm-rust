pub mod actor;

use tauri::AppHandle;
use tauri::Manager;
use std::path::PathBuf;
use rusqlite::Connection;

pub fn get_db_path(app_handle: &AppHandle) -> PathBuf {
    let app_dir = app_handle.path().app_data_dir().expect("Failed to get app data dir");
    if !app_dir.exists() {
        let _ = std::fs::create_dir_all(&app_dir);
    }
    app_dir.join("mitm.db")
}

pub fn init_database(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let db_path = get_db_path(app_handle);
    let mut conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;

         CREATE TABLE IF NOT EXISTS history (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             uuid TEXT NOT NULL UNIQUE,
             method TEXT NOT NULL,
             url TEXT NOT NULL,
             host TEXT NOT NULL,
             status_code INTEGER NOT NULL,
             request_headers TEXT NOT NULL,
             response_headers TEXT NOT NULL,
             request_body TEXT NOT NULL,
             response_body TEXT NOT NULL,
             phase TEXT NOT NULL,
             duration_ms INTEGER,
             created_at DATETIME DEFAULT CURRENT_TIMESTAMP
         );

         CREATE INDEX IF NOT EXISTS idx_history_created_at ON history(created_at DESC);
         CREATE INDEX IF NOT EXISTS idx_history_method ON history(method);
         CREATE INDEX IF NOT EXISTS idx_history_status ON history(status_code);
         CREATE INDEX IF NOT EXISTS idx_history_host ON history(host);
        "
    ).map_err(|e| e.to_string())?;
    
    Ok(db_path)
}
