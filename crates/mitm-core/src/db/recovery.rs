use std::path::{Path, PathBuf};
use crate::events::EventBus;
use super::InitError;

pub fn run_migrations(db_path: &PathBuf, event_bus: Option<&EventBus>) -> Result<(), InitError> {
    super::init_database(db_path, event_bus)
}

/// Copies the database aside as `<name>.bak_<unix secs>`, deletes it with its WAL and SHM files,
/// and migrates a fresh one. Returns the backup path, or `None` when there was nothing to back up.
pub fn backup_and_reset(db_path: &PathBuf, event_bus: Option<&EventBus>) -> Result<Option<PathBuf>, InitError> {
    let backup = if db_path.exists() {
        let backup = sibling(db_path, &format!(".bak_{}", chrono::Utc::now().timestamp()));
        std::fs::copy(db_path, &backup).map_err(|e| InitError::Io(format!("Failed to back up database: {}", e)))?;
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(sibling(db_path, suffix));
        }
        Some(backup)
    } else {
        None
    };
    super::init_database(db_path, event_bus)?;
    Ok(backup)
}

pub fn export_database(db_path: &Path, destination: &Path) -> Result<(), String> {
    if !db_path.exists() {
        return Err("Database file does not exist yet".to_string());
    }
    std::fs::copy(db_path, destination).map_err(|e| format!("Failed to export database: {}", e))?;
    Ok(())
}

fn sibling(db_path: &Path, suffix: &str) -> PathBuf {
    let mut name = db_path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    db_path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mitm-recovery-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn backup_and_reset_keeps_a_copy_and_migrates_a_fresh_db() {
        let dir = temp_dir();
        let db = dir.join("mitm.db");
        std::fs::write(&db, b"not a database").unwrap();
        std::fs::write(dir.join("mitm.db-wal"), b"x").unwrap();

        let backup = backup_and_reset(&db, None).unwrap().unwrap();
        assert_eq!(std::fs::read(&backup).unwrap(), b"not a database");
        assert!(backup.file_name().unwrap().to_string_lossy().starts_with("mitm.db.bak_"));
        assert!(rusqlite::Connection::open(&db).unwrap().prepare("SELECT 1 FROM requests").is_ok());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn export_copies_the_file_and_rejects_a_missing_db() {
        let dir = temp_dir();
        let db = dir.join("mitm.db");
        assert!(export_database(&db, &dir.join("out.db")).is_err());
        std::fs::write(&db, b"data").unwrap();
        export_database(&db, &dir.join("out.db")).unwrap();
        assert_eq!(std::fs::read(dir.join("out.db")).unwrap(), b"data");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
