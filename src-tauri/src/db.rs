use tauri_plugin_sql::{Migration, MigrationKind};
use tauri::AppHandle;
use tauri::Manager;
use std::path::PathBuf;

pub fn get_db_path(app_handle: &AppHandle) -> PathBuf {
    app_handle.path().app_data_dir().expect("Failed to get app data dir").join("mitm.db")
}

pub fn init_database(app_handle: &AppHandle) -> Result<(), String> {
    let db_path = get_db_path(app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;

    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS history (
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
        );
        CREATE TABLE IF NOT EXISTS repeater_groups (
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
        );
        CREATE TABLE IF NOT EXISTS environments (
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
        );
        CREATE TABLE IF NOT EXISTS app_state (
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
        );
    ").map_err(|e| e.to_string())?;

    Ok(())
}

pub fn get_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "initial history table",
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
        },
        Migration {
            version: 2,
            description: "repeater and organization tables",
            sql: "CREATE TABLE IF NOT EXISTS repeater_groups (
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
            kind: MigrationKind::Up,
        },
        Migration {
            version: 3,
            description: "environments and variables",
            sql: "CREATE TABLE IF NOT EXISTS environments (
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
            kind: MigrationKind::Up,
        },
        Migration {
            version: 4,
            description: "app state and replacements",
            sql: "CREATE TABLE IF NOT EXISTS app_state (
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
            kind: MigrationKind::Up,
        },
    ]
}
