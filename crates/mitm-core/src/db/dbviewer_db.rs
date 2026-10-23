//! Storage for the DB Viewer's connections and saved queries. The rows hold the viewer's own JSON, so this
//! crate doesn't need to know their shape beyond the key fields.
use std::path::PathBuf;
use rusqlite::Connection;
use serde_json::Value;
use crate::db::Db;

pub struct StoredRow {
    pub key: Vec<String>,
    pub data: String,
}

fn load(db: &Db, sql: &str) -> Result<Vec<String>, String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| row.get(0)).map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

pub fn load_connections(db: &Db) -> Result<Vec<String>, String> {
    load(db, "SELECT data FROM dbviewer_connections ORDER BY position")
}

pub fn load_queries(db: &Db) -> Result<Vec<String>, String> {
    load(db, "SELECT data FROM dbviewer_queries ORDER BY position")
}

pub fn save_connections(db: &Db, rows: &[StoredRow]) -> Result<(), String> {
    replace(db, "dbviewer_connections", "name", rows)
}

pub fn save_queries(db: &Db, rows: &[StoredRow]) -> Result<(), String> {
    replace(db, "dbviewer_queries", "connection, name", rows)
}

fn replace(db: &Db, table: &str, key_columns: &str, rows: &[StoredRow]) -> Result<(), String> {
    let mut conn = db.conn().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    insert_all(&tx, table, key_columns, rows)?;
    tx.commit().map_err(|e| e.to_string())
}

fn insert_all(conn: &Connection, table: &str, key_columns: &str, rows: &[StoredRow]) -> Result<(), String> {
    conn.execute(&format!("DELETE FROM {table}"), []).map_err(|e| e.to_string())?;
    let placeholders = vec!["?"; key_columns.split(',').count()].join(", ");
    let sql = format!("INSERT OR REPLACE INTO {table} ({key_columns}, position, data) VALUES ({placeholders}, ?, ?)");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    for (position, row) in rows.iter().enumerate() {
        let mut values: Vec<rusqlite::types::Value> = row.key.iter().cloned().map(Into::into).collect();
        values.push((position as i64).into());
        values.push(row.data.clone().into());
        stmt.execute(rusqlite::params_from_iter(values)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn legacy_dir() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("dbviewer"))
}

fn read_array(file: &str) -> Vec<Value> {
    let Some(path) = legacy_dir().map(|d| d.join(file)) else { return Vec::new() };
    let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
    match serde_json::from_str(&text) {
        Ok(Value::Array(items)) => items,
        _ => Vec::new(),
    }
}

fn string_field(item: &Value, field: &str) -> Option<String> {
    item.get(field)?.as_str().map(str::to_string)
}

/// Brings the standalone viewer's connections.json and queries.json into the app database. The files stay as
/// they are, and only the real app database is touched so tests and temp databases don't read the user's files.
pub fn import_json_files(conn: &Connection) -> Result<(), String> {
    if conn.path().map(PathBuf::from) != Some(crate::db::get_default_db_path()) {
        return Ok(());
    }
    import_rows(conn, &read_array("connections.json"), &read_array("queries.json"))
}

fn import_rows(conn: &Connection, connections: &[Value], queries: &[Value]) -> Result<(), String> {
    let connection_rows: Vec<StoredRow> = connections
        .iter()
        .filter_map(|c| Some(StoredRow { key: vec![string_field(c, "name")?], data: c.to_string() }))
        .collect();
    let query_rows: Vec<StoredRow> = queries
        .iter()
        .filter_map(|q| Some(StoredRow { key: vec![string_field(q, "connection")?, string_field(q, "name")?], data: q.to_string() }))
        .collect();
    insert_all(conn, "dbviewer_connections", "name", &connection_rows)?;
    insert_all(conn, "dbviewer_queries", "connection, name", &query_rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> (Db, PathBuf) {
        let dir = std::env::temp_dir().join(format!("mitm-dbviewer-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = Db::new(dir.join("test.db"));
        crate::db::init_database(&db, None).unwrap();
        (db, dir)
    }

    fn row(key: &[&str], data: &str) -> StoredRow {
        StoredRow { key: key.iter().map(|k| k.to_string()).collect(), data: data.into() }
    }

    #[test]
    fn saves_and_loads_in_order() {
        let (db, dir) = temp_db();
        save_connections(&db, &[row(&["b"], "{\"name\":\"b\"}"), row(&["a"], "{\"name\":\"a\"}")]).unwrap();
        assert_eq!(load_connections(&db).unwrap(), vec!["{\"name\":\"b\"}", "{\"name\":\"a\"}"]);
        save_queries(&db, &[row(&["c", "q1"], "1"), row(&["c", "q2"], "2")]).unwrap();
        save_queries(&db, &[row(&["c", "q2"], "2")]).unwrap();
        assert_eq!(load_queries(&db).unwrap(), vec!["2"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn imports_legacy_json_rows() {
        let (db, dir) = temp_db();
        let connections: Vec<Value> = serde_json::from_str(r#"[{"name":"prod","host":"h"},{"nameless":1}]"#).unwrap();
        let queries: Vec<Value> = serde_json::from_str(r#"[{"connection":"prod","name":"q","database":"d","sql":"select 1"}]"#).unwrap();
        import_rows(&db.conn().unwrap(), &connections, &queries).unwrap();
        assert_eq!(load_connections(&db).unwrap().len(), 1);
        assert_eq!(load_queries(&db).unwrap().len(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
