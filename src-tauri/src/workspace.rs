use tauri::AppHandle;
use crate::db;
use crate::models::{GlobalVariable, Replacement};

#[tauri::command]
pub async fn update_variable(app_handle: AppHandle, id: String, updates: GlobalVariable) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    conn.execute(
        "UPDATE variables SET name = ?, active_index = ?, order_index = ? WHERE id = ?",
        rusqlite::params![updates.name, updates.active_index, updates.order_index, id],
    ).map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM variable_values WHERE variable_id = ?", [&id]).map_err(|e| e.to_string())?;
    for val in updates.values {
        conn.execute(
            "INSERT INTO variable_values (id, variable_id, name, value) VALUES (?, ?, ?, ?)",
            rusqlite::params![val.id, id, val.name, val.value],
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn create_variable(app_handle: AppHandle, variable: GlobalVariable) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    conn.execute(
        "INSERT INTO variables (id, environment_id, name, active_index, order_index) VALUES (?, ?, ?, ?, ?)",
        rusqlite::params![variable.id, variable.environment_id, variable.name, variable.active_index, variable.order_index],
    ).map_err(|e| e.to_string())?;

    for val in variable.values {
        conn.execute(
            "INSERT INTO variable_values (id, variable_id, name, value) VALUES (?, ?, ?, ?)",
            rusqlite::params![val.id, variable.id, val.name, val.value],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_variable(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM variables WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn create_environment(app_handle: AppHandle, id: String, name: String) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("INSERT INTO environments (id, name, is_active) VALUES (?, ?, 0)", [id, name]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_environment(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM environments WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_active_environment(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("UPDATE environments SET is_active = 0", []).map_err(|e| e.to_string())?;
    conn.execute("UPDATE environments SET is_active = 1 WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn save_replacements_bulk(app_handle: AppHandle, replacements: Vec<Replacement>) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    for r in replacements {
        conn.execute(
            "INSERT OR REPLACE INTO replacements (id, type, pattern, replacement, description, is_active, order_index, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, strftime('%s', 'now'))",
            rusqlite::params![r.id, r.r_type, r.pattern, r.replacement, r.description, r.is_active, r.order_index],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_replacement(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM replacements WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn update_replacement_order(app_handle: AppHandle, items: Vec<Replacement>) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    for r in items {
        conn.execute("UPDATE replacements SET order_index = ?, updated_at = strftime('%s', 'now') WHERE id = ?", rusqlite::params![r.order_index, r.id]).map_err(|e| e.to_string())?;
    }
    Ok(())
}
