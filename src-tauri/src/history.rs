use tauri::AppHandle;
use crate::db;

#[tauri::command]
pub async fn clear_history(app_handle: AppHandle) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM history", []).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_history_item(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM history WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}
