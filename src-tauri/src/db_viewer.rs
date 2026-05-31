use tauri::AppHandle;
use crate::db;
use serde_json::{json, Value};

#[tauri::command]
pub async fn get_database_tables(app_handle: AppHandle) -> Result<Vec<String>, String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
    ).map_err(|e| e.to_string())?;
    
    let tables = stmt.query_map([], |row| {
        row.get::<_, String>(0)
    }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    
    Ok(tables)
}

#[tauri::command]
pub async fn get_table_data(app_handle: AppHandle, table_name: String, limit: i32) -> Result<Value, String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    // Sanitize table name to prevent SQL injection
    if !table_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Invalid table name".to_string());
    }
    
    // Get column names
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table_name))
        .map_err(|e| e.to_string())?;
    
    let columns: Vec<String> = stmt.query_map([], |row| {
        row.get::<_, String>(1) // column name is at index 1
    }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    
    if columns.is_empty() {
        return Err("Table not found or has no columns".to_string());
    }
    
    // Get row count
    let row_count: i32 = conn.query_row(
        &format!("SELECT COUNT(*) FROM {}", table_name),
        [],
        |row| row.get(0)
    ).map_err(|e| e.to_string())?;
    
    // Get data
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM {} LIMIT ?",
        columns.join(","),
        table_name
    )).map_err(|e| e.to_string())?;
    
    let mut rows = Vec::new();
    let mut row_iter = stmt.query([&limit.to_string()])
        .map_err(|e| e.to_string())?;
    
    while let Some(row) = row_iter.next().map_err(|e| e.to_string())? {
        let mut row_data = json!({});
        for (idx, col_name) in columns.iter().enumerate() {
            let value: Result<String, _> = row.get(idx);
            match value {
                Ok(v) => {
                    // Try to parse as JSON for better display
                    if let Ok(json_val) = serde_json::from_str::<Value>(&v) {
                        row_data[col_name] = json_val;
                    } else {
                        row_data[col_name] = Value::String(v);
                    }
                }
                Err(_) => {
                    row_data[col_name] = Value::Null;
                }
            }
        }
        rows.push(row_data);
    }
    
    Ok(json!({
        "table": table_name,
        "columns": columns,
        "row_count": row_count,
        "rows": rows,
        "limit": limit
    }))
}
