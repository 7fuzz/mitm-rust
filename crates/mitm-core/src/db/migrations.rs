use rusqlite::Connection;
use crate::events::{CoreEvent, EventBus, MigrationProgressPayload};

pub struct MigrationSpec {
    pub version: u32,
    pub name: &'static str,
    pub sql: &'static str,
    /// Data changes SQL can't express, run in the migration's transaction after its SQL.
    pub after: Option<fn(&Connection) -> Result<(), String>>,
}

pub const MIGRATIONS: &[MigrationSpec] = &[
    MigrationSpec {
        version: 1,
        name: "20260825_0001_init",
        sql: include_str!("../../migrations/20260825_0001_init.sql"),
        after: None,
    },
    MigrationSpec {
        version: 2,
        name: "20260825_0002_intercept_action",
        sql: include_str!("../../migrations/20260825_0002_intercept_action.sql"),
        after: None,
    },
    MigrationSpec {
        version: 3,
        name: "20260825_0003_repeaters",
        sql: include_str!("../../migrations/20260825_0003_repeaters.sql"),
        after: None,
    },
    MigrationSpec {
        version: 4,
        name: "20260825_0004_workspaces_and_collections",
        sql: include_str!("../../migrations/20260825_0004_workspaces_and_collections.sql"),
        after: None,
    },
    MigrationSpec {
        version: 5,
        name: "20260825_0005_fix_environments_schema",
        sql: include_str!("../../migrations/20260825_0005_fix_environments_schema.sql"),
        after: None,
    },
    MigrationSpec {
        version: 6,
        name: "20260827_0006_separate_body_buffers",
        sql: include_str!("../../migrations/20260827_0006_separate_body_buffers.sql"),
        after: None,
    },
    MigrationSpec {
        version: 7,
        name: "20260827_0007_drop_body_content",
        sql: include_str!("../../migrations/20260827_0007_drop_body_content.sql"),
        after: None,
    },
    MigrationSpec {
        version: 8,
        name: "20260831_0008_rewrite_rules",
        sql: include_str!("../../migrations/20260831_0008_rewrite_rules.sql"),
        after: None,
    },
    MigrationSpec {
        version: 9,
        name: "20260831_0009_history_flags",
        sql: include_str!("../../migrations/20260831_0009_history_flags.sql"),
        after: None,
    },
    MigrationSpec {
        version: 10,
        name: "20260831_0010_webhooks",
        sql: include_str!("../../migrations/20260831_0010_webhooks.sql"),
        after: None,
    },
    MigrationSpec {
        version: 11,
        name: "20260831_0011_websockets",
        sql: include_str!("../../migrations/20260831_0011_websockets.sql"),
        after: None,
    },
    MigrationSpec {
        version: 12,
        name: "20261001_0012_listener_source",
        sql: include_str!("../../migrations/20261001_0012_listener_source.sql"),
        after: None,
    },
    MigrationSpec {
        version: 13,
        name: "20261002_0013_source_scoping",
        sql: include_str!("../../migrations/20261002_0013_source_scoping.sql"),
        after: None,
    },
    MigrationSpec {
        version: 14,
        name: "20261002_0014_history_timestamps",
        sql: include_str!("../../migrations/20261002_0014_history_timestamps.sql"),
        after: None,
    },
    MigrationSpec {
        version: 15,
        name: "20261003_0015_fuzzer",
        sql: include_str!("../../migrations/20261003_0015_fuzzer.sql"),
        after: None,
    },
    MigrationSpec {
        version: 16,
        name: "20261003_0016_repeater_workspace",
        sql: include_str!("../../migrations/20261003_0016_repeater_workspace.sql"),
        after: None,
    },
    MigrationSpec {
        version: 17,
        name: "20261004_0017_integer_ids",
        sql: include_str!("../../migrations/20261004_0017_integer_ids.sql"),
        after: None,
    },
    MigrationSpec {
        version: 18,
        name: "20261004_0018_drop_legacy_proxy_address",
        sql: include_str!("../../migrations/20261004_0018_drop_legacy_proxy_address.sql"),
        after: None,
    },
    MigrationSpec {
        version: 19,
        name: "20261005_0019_request_steps",
        sql: include_str!("../../migrations/20261005_0019_request_steps.sql"),
        after: None,
    },
    MigrationSpec {
        version: 20,
        name: "20261008_0020_fuzz_write_through",
        sql: include_str!("../../migrations/20261008_0020_fuzz_write_through.sql"),
        after: None,
    },
    MigrationSpec {
        version: 21,
        name: "20261012_0021_credentials",
        sql: include_str!("../../migrations/20261012_0021_credentials.sql"),
        after: None,
    },
    MigrationSpec {
        version: 22,
        name: "20261013_0022_vaults",
        sql: include_str!("../../migrations/20261013_0022_vaults.sql"),
        after: Some(crate::credentials::migrate_into_vaults),
    },
    MigrationSpec {
        version: 23,
        name: "20261013_0023_credential_tags",
        sql: include_str!("../../migrations/20261013_0023_credential_tags.sql"),
        after: None,
    },
    MigrationSpec {
        version: 24,
        name: "20261013_0024_body_store",
        sql: include_str!("../../migrations/20261013_0024_body_store.sql"),
        after: Some(crate::db::bodies::migrate_history_bodies),
    },
];

pub fn run_all(event_bus: Option<&EventBus>, conn: &Connection) -> Result<(), String> {
    let emit_progress = |payload: MigrationProgressPayload| {
        if let Some(bus) = event_bus {
            bus.emit(CoreEvent::DbMigrationProgress(payload));
        }
    };

    let pragma_user_version: u32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap_or(0);

    let latest_version = MIGRATIONS.last().map(|m| m.version).unwrap_or(0);

    // A user_version above our latest one without the core tables means another tool wrote this file;
    // report it as a mismatch so the recovery screen opens.
    let requests_exists = conn.prepare("SELECT 1 FROM requests LIMIT 1").is_ok();
    if pragma_user_version > latest_version && !requests_exists {
        let err_msg = format!(
            "Database version mismatch detected (PRAGMA user_version is {} but latest schema is {}). Essential tables are missing.",
            pragma_user_version, latest_version
        );
        emit_progress(MigrationProgressPayload {
            step: 0,
            total: MIGRATIONS.len(),
            name: "schema_mismatch".to_string(),
            status: "Database Version Conflict Detected".to_string(),
            is_complete: false,
            has_error: true,
            error_message: Some(err_msg.clone()),
        });
        return Err(err_msg);
    }

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
            emit_progress(MigrationProgressPayload {
                step: idx + 1,
                total,
                name: m.name.to_string(),
                status: format!("Applying migration {}...", m.name),
                is_complete: false,
                has_error: false,
                error_message: None,
            });

            // One transaction per migration so a copy-then-drop can't be half applied
            conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;

            // Execute SQL statements individually to safely handle ALTER TABLE errors
            for statement in m.sql.split(';') {
                let trimmed = statement.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Err(err) = conn.execute(trimmed, []) {
                    let err_msg = err.to_string();
                    if !err_msg.contains("duplicate column name") && !err_msg.contains("already exists") {
                        let full_err = format!("Migration {} failed: {}", m.name, err_msg);
                        emit_progress(MigrationProgressPayload {
                            step: idx + 1,
                            total,
                            name: m.name.to_string(),
                            status: "Migration Error".to_string(),
                            is_complete: false,
                            has_error: true,
                            error_message: Some(full_err.clone()),
                        });
                        let _ = conn.execute_batch("ROLLBACK");
                        return Err(full_err);
                    }
                }
            }

            if let Some(after) = m.after {
                if let Err(err) = after(conn) {
                    let full_err = format!("Migration {} failed: {}", m.name, err);
                    emit_progress(MigrationProgressPayload {
                        step: idx + 1,
                        total,
                        name: m.name.to_string(),
                        status: "Migration Error".to_string(),
                        is_complete: false,
                        has_error: true,
                        error_message: Some(full_err.clone()),
                    });
                    let _ = conn.execute_batch("ROLLBACK");
                    return Err(full_err);
                }
            }

            let marked = conn
                .execute(
                    "INSERT OR REPLACE INTO _schema_migrations (name) VALUES (?)",
                    [m.name],
                )
                .and_then(|_| conn.execute_batch("COMMIT"));
            if let Err(e) = marked {
                let _ = conn.execute_batch("ROLLBACK");
                return Err(e.to_string());
            }
        }
    }

    let _ = conn.execute(&format!("PRAGMA user_version = {}", latest_version), []);

    emit_progress(MigrationProgressPayload {
        step: total,
        total,
        name: "complete".to_string(),
        status: "Database migrations completed successfully.".to_string(),
        is_complete: true,
        has_error: false,
        error_message: None,
    });

    eprintln!("[DB Migration] All database migrations verified and up to date.");
    Ok(())
}

