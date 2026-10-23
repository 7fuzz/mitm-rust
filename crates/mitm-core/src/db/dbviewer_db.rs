//! Storage for the DB Viewer's saved queries. The rows hold the viewer's own JSON, so this crate
//! doesn't need to know their shape beyond the key fields. Connections live in credentials.
use std::path::PathBuf;
use rusqlite::Connection;
use serde_json::Value;
use crate::db::Db;

pub struct StoredQuery {
    pub connection: String,
    pub name: String,
    pub data: String,
}

pub fn load_queries(db: &Db) -> Result<Vec<String>, String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT data FROM dbviewer_queries ORDER BY position").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| row.get(0)).map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

pub fn save_queries(db: &Db, rows: &[StoredQuery]) -> Result<(), String> {
    let mut conn = db.conn().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    insert_all(&tx, rows)?;
    tx.commit().map_err(|e| e.to_string())
}

fn insert_all(conn: &Connection, rows: &[StoredQuery]) -> Result<(), String> {
    conn.execute("DELETE FROM dbviewer_queries", []).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("INSERT OR REPLACE INTO dbviewer_queries (connection, name, position, data) VALUES (?, ?, ?, ?)")
        .map_err(|e| e.to_string())?;
    for (position, row) in rows.iter().enumerate() {
        stmt.execute(rusqlite::params![row.connection, row.name, position as i64, row.data]).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn legacy_array(db_path: &std::path::Path, file: &str) -> Vec<Value> {
    if db_path != crate::db::get_default_db_path() {
        return Vec::new();
    }
    let Some(path) = dirs::data_dir().map(|d| d.join("dbviewer").join(file)) else { return Vec::new() };
    let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
    match serde_json::from_str(&text) {
        Ok(Value::Array(items)) => items,
        _ => Vec::new(),
    }
}

/// The standalone viewer's connections.json entries, for the real app database only so tests and temp
/// databases never read the user's files.
pub fn legacy_connections(db: &Db) -> Vec<Value> {
    legacy_array(db.path(), "connections.json")
}

/// Brings the standalone viewer's queries.json into the app database. The file stays as it is.
pub fn import_json_files(conn: &Connection) -> Result<(), String> {
    let queries = conn.path().map(PathBuf::from).map(|p| legacy_array(&p, "queries.json")).unwrap_or_default();
    import_queries(conn, &queries)
}

fn import_queries(conn: &Connection, queries: &[Value]) -> Result<(), String> {
    let field = |q: &Value, name: &str| q.get(name).and_then(Value::as_str).map(str::to_string);
    let rows: Vec<StoredQuery> = queries
        .iter()
        .filter_map(|q| Some(StoredQuery { connection: field(q, "connection")?, name: field(q, "name")?, data: q.to_string() }))
        .collect();
    insert_all(conn, &rows)
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

    fn query(connection: &str, name: &str, data: &str) -> StoredQuery {
        StoredQuery { connection: connection.into(), name: name.into(), data: data.into() }
    }

    #[test]
    fn saves_and_loads_in_order() {
        let (db, dir) = temp_db();
        save_queries(&db, &[query("c", "q1", "1"), query("c", "q2", "2")]).unwrap();
        assert_eq!(load_queries(&db).unwrap(), vec!["1", "2"]);
        save_queries(&db, &[query("c", "q2", "2")]).unwrap();
        assert_eq!(load_queries(&db).unwrap(), vec!["2"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn imports_legacy_json_rows() {
        let (db, dir) = temp_db();
        let queries: Vec<Value> =
            serde_json::from_str(r#"[{"connection":"prod","name":"q","database":"d","sql":"select 1"},{"nameless":1}]"#).unwrap();
        import_queries(&db.conn().unwrap(), &queries).unwrap();
        assert_eq!(load_queries(&db).unwrap().len(), 1);
        assert!(legacy_connections(&db).is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
