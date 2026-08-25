use rusqlite::Connection;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub struct MigrationSpec {
    pub version: u32,
    pub name: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[MigrationSpec] = &[
    MigrationSpec {
        version: 1,
        name: "20260825_0001_init",
        sql: include_str!("../../migrations/20260825_0001_init.sql"),
    },
    MigrationSpec {
        version: 2,
        name: "20260825_0002_intercept_action",
        sql: include_str!("../../migrations/20260825_0002_intercept_action.sql"),
    },
    MigrationSpec {
        version: 3,
        name: "20260825_0003_repeaters",
        sql: include_str!("../../migrations/20260825_0003_repeaters.sql"),
    },
    MigrationSpec {
        version: 4,
        name: "20260825_0004_workspaces_and_collections",
        sql: include_str!("../../migrations/20260825_0004_workspaces_and_collections.sql"),
    },
];

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MigrationProgressPayload {
    pub step: usize,
    pub total: usize,
    pub name: String,
    pub status: String,
    pub is_complete: bool,
    pub has_error: bool,
    pub error_message: Option<String>,
}

pub fn run_all(app_handle: &AppHandle, conn: &Connection) -> Result<(), String> {
    // 1. Check if legacy user_version conflict exists (> LATEST_VERSION)
    let pragma_user_version: u32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap_or(0);

    let latest_version = MIGRATIONS.last().map(|m| m.version).unwrap_or(0);

    // If pragma_user_version is corrupted or artificially inflated above latest schema (e.g. version 13 from plugin conflict)
    // AND essential tables like 'repeaters' are missing, raise a explicit schema mismatch error so recovery modal activates!
    let repeaters_exists = conn.prepare("SELECT 1 FROM repeaters LIMIT 1").is_ok();
    if pragma_user_version > latest_version && !repeaters_exists {
        let err_msg = format!(
            "Database version mismatch detected (PRAGMA user_version is {} but latest schema is {}). Essential tables are missing.",
            pragma_user_version, latest_version
        );
        let _ = app_handle.emit(
            "db-migration-progress",
            MigrationProgressPayload {
                step: 0,
                total: MIGRATIONS.len(),
                name: "schema_mismatch".to_string(),
                status: "Database Version Conflict Detected".to_string(),
                is_complete: false,
                has_error: true,
                error_message: Some(err_msg.clone()),
            },
        );
        return Err(err_msg);
    }

    // 2. Ensure internal migrations table exists
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _schema_migrations (
            name TEXT PRIMARY KEY,
            executed_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );"
    ).map_err(|e| e.to_string())?;

    let total = MIGRATIONS.len();

    for (idx, m) in MIGRATIONS.iter().enumerate() {
        let is_applied: bool = conn
            .query_row(
                "SELECT 1 FROM _schema_migrations WHERE name = ?",
                [m.name],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if !is_applied {
            eprintln!("[DB Migration] Applying Migration v{} ({})", m.version, m.name);
            let _ = app_handle.emit(
                "db-migration-progress",
                MigrationProgressPayload {
                    step: idx + 1,
                    total,
                    name: m.name.to_string(),
                    status: format!("Applying migration {}...", m.name),
                    is_complete: false,
                    has_error: false,
                    error_message: None,
                },
            );

            if let Err(err) = conn.execute_batch(m.sql) {
                let err_msg = err.to_string();
                if !err_msg.contains("duplicate column name") && !err_msg.contains("already exists") {
                    let full_err = format!("Migration {} failed: {}", m.name, err_msg);
                    let _ = app_handle.emit(
                        "db-migration-progress",
                        MigrationProgressPayload {
                            step: idx + 1,
                            total,
                            name: m.name.to_string(),
                            status: "Migration Error".to_string(),
                            is_complete: false,
                            has_error: true,
                            error_message: Some(full_err.clone()),
                        },
                    );
                    return Err(full_err);
                }
            }

            conn.execute(
                "INSERT OR REPLACE INTO _schema_migrations (name) VALUES (?)",
                [m.name],
            ).map_err(|e| e.to_string())?;
        }
    }

    // Reset user_version to match latest migration version for consistency
    let _ = conn.execute(&format!("PRAGMA user_version = {}", latest_version), []);

    let _ = app_handle.emit(
        "db-migration-progress",
        MigrationProgressPayload {
            step: total,
            total,
            name: "complete".to_string(),
            status: "Database migrations completed successfully.".to_string(),
            is_complete: true,
            has_error: false,
            error_message: None,
        },
    );

    eprintln!("[DB Migration] All database migrations verified and up to date.");
    Ok(())
}
