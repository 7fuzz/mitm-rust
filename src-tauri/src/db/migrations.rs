use rusqlite::Connection;

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
];

pub fn run_all(conn: &Connection) -> Result<(), String> {
    let current_version: u32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap_or(0);

    eprintln!("[DB Migration] Checking database migrations (current schema version: {})", current_version);

    for m in MIGRATIONS {
        if current_version < m.version {
            eprintln!("[DB Migration] Applying Migration v{} ({})", m.version, m.name);
            if let Err(err) = conn.execute_batch(m.sql) {
                // Ignore harmless duplicate column errors on legacy DBs
                if !err.to_string().contains("duplicate column name") {
                    return Err(format!("Migration {} failed: {}", m.name, err));
                }
            }
            conn.execute(&format!("PRAGMA user_version = {}", m.version), [])
                .map_err(|e| e.to_string())?;
        }
    }

    let latest = MIGRATIONS.last().map(|m| m.version).unwrap_or(0);
    eprintln!("[DB Migration] Database up to date at schema version {}", latest);
    Ok(())
}
