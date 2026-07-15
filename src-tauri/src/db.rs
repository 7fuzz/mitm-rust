use tauri::AppHandle;
use tauri::Manager;
use std::path::PathBuf;
use rusqlite::Connection;

pub fn get_db_path(app_handle: &AppHandle) -> PathBuf {
    app_handle.path().app_data_dir().expect("Failed to get app data dir").join("mitm.db")
}

pub fn init_database(app_handle: &AppHandle) -> Result<(), String> {
    let db_path = get_db_path(app_handle);
    let mut conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    apply_migrations(&mut conn).map_err(|e| e.to_string())?;
    
    Ok(())
}

fn apply_migrations(conn: &mut Connection) -> rusqlite::Result<()> {
    // 1. Get current version
    let current_version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    
    let migrations = vec![
        // Version 1: Initial History
        "CREATE TABLE IF NOT EXISTS history (
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
            duration_ms INTEGER,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );",
        // Version 2: Repeater Groups & Requests
        "CREATE TABLE IF NOT EXISTS repeater_groups (
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
        );",
        // Version 3: Environments & Variables
        "CREATE TABLE IF NOT EXISTS environments (
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
        );",
        // Version 4: App State & Replacements
        "CREATE TABLE IF NOT EXISTS app_state (
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
        );",
        // Version 5: Repeater Group Extractions
        "ALTER TABLE repeater_groups ADD COLUMN extract TEXT;",
        // Version 6: WebSocket Message Logging
        "CREATE TABLE IF NOT EXISTS websocket_messages (
            id TEXT PRIMARY KEY,
            connection_id TEXT NOT NULL,
            direction TEXT NOT NULL,
            msg_type TEXT NOT NULL,
            payload TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            is_intercepted INTEGER DEFAULT 0,
            FOREIGN KEY(connection_id) REFERENCES history(id) ON DELETE CASCADE
        );",
    ];

    let target_version = migrations.len() as i32;
    
    if current_version < target_version {
        let tx = conn.transaction()?;
        
        for (idx, sql) in migrations.iter().enumerate() {
            let version = (idx + 1) as i32;
            if version > current_version {
                tx.execute_batch(sql)?;
            }
        }
        
        tx.pragma_update(None, "user_version", target_version)?;
        tx.commit()?;
    }
    
    Ok(())
}
