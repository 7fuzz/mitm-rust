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

fn delete_group_and_descendants_tx(tx: &rusqlite::Transaction, group_id: &str) -> Result<(), String> {
    let mut stmt = tx.prepare("SELECT id FROM repeater_groups WHERE parent_id = ?").map_err(|e| e.to_string())?;
    let child_ids: Vec<String> = stmt.query_map([group_id], |row| row.get(0)).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
    drop(stmt);
    for cid in &child_ids {
        delete_group_and_descendants_tx(tx, cid)?;
    }
    tx.execute("DELETE FROM request_bodies WHERE request_id IN (SELECT id FROM repeater_requests WHERE group_id = ?)", [group_id]).ok();
    tx.execute("DELETE FROM repeater_requests WHERE group_id = ?", [group_id]).map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM environment_groups WHERE group_id = ?", [group_id]).map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM repeater_groups WHERE id = ?", [group_id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_environment(
    app_handle: AppHandle,
    id: String,
    delete_linked_collections: Option<bool>
) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let mut conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    if delete_linked_collections.unwrap_or(false) {
        let mut stmt = tx.prepare("SELECT group_id FROM environment_groups WHERE environment_id = ?").map_err(|e| e.to_string())?;
        let group_ids: Vec<String> = stmt.query_map([&id], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        drop(stmt);

        for g_id in group_ids {
            delete_group_and_descendants_tx(&tx, &g_id)?;
        }
    }

    tx.execute("DELETE FROM environment_groups WHERE environment_id = ?", [&id]).map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM variables WHERE environment_id = ?", [&id]).map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM environments WHERE id = ?", [&id]).map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_active_environment(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("UPDATE environments SET is_active = 0", []).map_err(|e| e.to_string())?;
    conn.execute("UPDATE environments SET is_active = 1 WHERE id = ?", [id.clone()]).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO app_state (key, value) VALUES ('active_env_id', ?)", [id]).map_err(|e| e.to_string())?;
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
