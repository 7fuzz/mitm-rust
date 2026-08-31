use tauri::AppHandle;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[tauri::command]
pub fn run_database_migrations(app_handle: AppHandle) -> Result<(), String> {
    crate::db::init_database(&app_handle)?;
    Ok(())
}

#[tauri::command]
pub fn backup_and_reset_database(app_handle: AppHandle) -> Result<String, String> {
    let db_path = crate::db::get_db_path(&app_handle);

    if db_path.exists() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let backup_file_name = format!("mitm.db.bak_{}", timestamp);
        let parent = db_path.parent().ok_or("Invalid DB path parent")?;
        let backup_path = parent.join(&backup_file_name);

        // Copy current database to backup file
        fs::copy(&db_path, &backup_path)
            .map_err(|e| format!("Failed to create database backup: {}", e))?;

        // Remove original db and associated WAL/SHM files
        let _ = fs::remove_file(&db_path);
        let _ = fs::remove_file(parent.join("mitm.db-wal"));
        let _ = fs::remove_file(parent.join("mitm.db-shm"));

        // Re-initialize clean database and apply migrations
        crate::db::init_database(&app_handle)?;

        Ok(format!("Database safely backed up to '{}' and re-initialized.", backup_file_name))
    } else {
        crate::db::init_database(&app_handle)?;
        Ok("Database re-initialized.".to_string())
    }
}

#[tauri::command]
pub fn export_database_file(app_handle: AppHandle, destination_path: String) -> Result<(), String> {
    let db_path = crate::db::get_db_path(&app_handle);

    if !db_path.exists() {
        return Err("Database file does not exist yet".to_string());
    }

    fs::copy(&db_path, destination_path)
        .map_err(|e| format!("Failed to export database: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn quit_application(app_handle: AppHandle) {
    app_handle.exit(0);
}
