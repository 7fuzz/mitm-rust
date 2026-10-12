use rusqlite::types::ValueRef;
use rusqlite::{params_from_iter, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::db::Db;

const GRID_CELL_MAX_CHARS: usize = 1000;
const MAX_PAGE_SIZE: i64 = 500;
const MAX_QUERY_ROWS: usize = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbColumn {
    pub name: String,
    pub decl_type: String,
    pub primary_key: bool,
    pub not_null: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbTable {
    pub name: String,
    pub row_count: i64,
    pub columns: Vec<DbColumn>,
    pub sql: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbRows {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub rowids: Option<Vec<i64>>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbQueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub truncated: bool,
    pub elapsed_ms: u64,
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn cell_to_json(value: ValueRef<'_>, max_chars: Option<usize>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => Value::from(i),
        ValueRef::Real(f) => Value::from(f),
        ValueRef::Text(bytes) => {
            let text = String::from_utf8_lossy(bytes);
            match max_chars {
                Some(max) if text.chars().count() > max => {
                    Value::String(format!("{}…", text.chars().take(max).collect::<String>()))
                }
                _ => Value::String(text.into_owned()),
            }
        }
        ValueRef::Blob(bytes) => Value::String(format!("[BLOB {} bytes]", bytes.len())),
    }
}

fn open_read_only(db: &Db) -> Result<Connection, String> {
    Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .map_err(|e| e.to_string())
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<DbColumn>, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({})", quote_ident(table)))
        .map_err(|e| e.to_string())?;
    let columns: Vec<DbColumn> = stmt
        .query_map([], |row| {
            Ok(DbColumn {
                name: row.get(1)?,
                decl_type: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                not_null: row.get::<_, i64>(3)? != 0,
                primary_key: row.get::<_, i64>(5)? != 0,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    if columns.is_empty() {
        return Err(format!("Unknown table '{}'", table));
    }
    Ok(columns)
}

fn table_has_rowid(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?",
        [table],
        |row| row.get::<_, Option<String>>(0),
    )
    .ok()
    .flatten()
    .map(|sql| !sql.to_uppercase().contains("WITHOUT ROWID"))
    .unwrap_or(false)
}

pub fn list_db_tables(db: &Db) -> Result<Vec<DbTable>, String> {
    let conn = open_read_only(db)?;
    let mut stmt = conn
        .prepare("SELECT name, COALESCE(sql, '') FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .map_err(|e| e.to_string())?;
    let tables: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut result = Vec::with_capacity(tables.len());
    for (name, sql) in tables {
        let row_count = conn
            .query_row(&format!("SELECT COUNT(*) FROM {}", quote_ident(&name)), [], |row| row.get(0))
            .unwrap_or(0);
        let columns = table_columns(&conn, &name).unwrap_or_default();
        result.push(DbTable { name, row_count, columns, sql });
    }
    Ok(result)
}

pub fn query_db_table(
    db: &Db,
    table: &str,
    offset: i64,
    limit: i64,
    order_by: Option<&str>,
    order_desc: Option<bool>,
    search: Option<&str>,
) -> Result<DbRows, String> {
    let conn = open_read_only(db)?;
    let columns = table_columns(&conn, table)?;
    let has_rowid = table_has_rowid(&conn, table);

    let search = search.map(|s| s.trim()).filter(|s| !s.is_empty());
    let mut params: Vec<String> = Vec::new();
    let where_clause = match search {
        Some(term) => {
            let escaped = term.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
            params.push(format!("%{}%", escaped));
            let conditions: Vec<String> = columns
                .iter()
                .map(|c| format!("CAST({} AS TEXT) LIKE ?1 ESCAPE '\\'", quote_ident(&c.name)))
                .collect();
            format!(" WHERE {}", conditions.join(" OR "))
        }
        None => String::new(),
    };

    let order_clause = match order_by {
        Some(col) if columns.iter().any(|c| c.name == col) => {
            format!(" ORDER BY {} {}", quote_ident(col), if order_desc.unwrap_or(false) { "DESC" } else { "ASC" })
        }
        Some(col) => return Err(format!("Unknown column '{}'", col)),
        None => String::new(),
    };

    let table_ident = quote_ident(table);
    let total: i64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM {}{}", table_ident, where_clause),
            params_from_iter(params.iter()),
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let column_list = columns.iter().map(|c| quote_ident(&c.name)).collect::<Vec<_>>().join(", ");
    let select_list = if has_rowid { format!("rowid, {}", column_list) } else { column_list };
    let sql = format!(
        "SELECT {} FROM {}{}{} LIMIT {} OFFSET {}",
        select_list,
        table_ident,
        where_clause,
        order_clause,
        limit.clamp(1, MAX_PAGE_SIZE),
        offset.max(0)
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let first_value = if has_rowid { 1 } else { 0 };
    let column_count = columns.len();
    let mut rowids = Vec::new();
    let mut rows = Vec::new();
    let mut query = stmt.query(params_from_iter(params.iter())).map_err(|e| e.to_string())?;
    while let Some(row) = query.next().map_err(|e| e.to_string())? {
        if has_rowid {
            rowids.push(row.get::<_, i64>(0).map_err(|e| e.to_string())?);
        }
        let mut values = Vec::with_capacity(column_count);
        for i in 0..column_count {
            let value = row.get_ref(first_value + i).map_err(|e| e.to_string())?;
            values.push(cell_to_json(value, Some(GRID_CELL_MAX_CHARS)));
        }
        rows.push(values);
    }

    Ok(DbRows {
        columns: columns.into_iter().map(|c| c.name).collect(),
        rows,
        rowids: if has_rowid { Some(rowids) } else { None },
        total,
    })
}

pub fn get_db_row(db: &Db, table: &str, rowid: i64) -> Result<Vec<Value>, String> {
    let conn = open_read_only(db)?;
    let columns = table_columns(&conn, table)?;
    let column_list = columns.iter().map(|c| quote_ident(&c.name)).collect::<Vec<_>>().join(", ");
    conn.query_row(
        &format!("SELECT {} FROM {} WHERE rowid = ?", column_list, quote_ident(table)),
        [rowid],
        |row| {
            (0..columns.len())
                .map(|i| row.get_ref(i).map(|v| cell_to_json(v, None)))
                .collect::<Result<Vec<_>, _>>()
        },
    )
    .map_err(|e| e.to_string())
}

pub fn run_db_query(db: &Db, sql: &str) -> Result<DbQueryResult, String> {
    let started = std::time::Instant::now();
    let conn = open_read_only(db)?;
    let mut stmt = conn.prepare(sql.trim()).map_err(|e| e.to_string())?;
    let columns: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();
    let column_count = columns.len();

    let mut rows = Vec::new();
    let mut truncated = false;
    let mut query = stmt.query([]).map_err(|e| e.to_string())?;
    while let Some(row) = query.next().map_err(|e| e.to_string())? {
        if rows.len() >= MAX_QUERY_ROWS {
            truncated = true;
            break;
        }
        let mut values = Vec::with_capacity(column_count);
        for i in 0..column_count {
            values.push(cell_to_json(row.get_ref(i).map_err(|e| e.to_string())?, Some(GRID_CELL_MAX_CHARS)));
        }
        rows.push(values);
    }

    Ok(DbQueryResult {
        columns,
        rows,
        truncated,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> (crate::db::Db, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("mitm-browser-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = crate::db::Db::new(dir.join("browser.db"));
        crate::db::init_database(&db, None).unwrap();
        (db, dir)
    }

    #[test]
    fn lists_tables_and_queries_table() {
        let (db, dir) = setup_test_db();
        let tables = list_db_tables(&db).unwrap();
        assert!(tables.iter().any(|t| t.name == "history"));
        assert!(tables.iter().any(|t| t.name == "intercept_rules"));

        let conn = crate::db::open(&db).unwrap();
        conn.execute("INSERT INTO history (id, method, url, host, status_code, request_headers, response_headers, request_body, response_body, phase) VALUES (1, 'GET', 'http://example.com/test', 'example.com', 200, '[]', '[]', '', '', 'done')", []).unwrap();
        conn.execute("INSERT INTO history (id, method, url, host, status_code, request_headers, response_headers, request_body, response_body, phase) VALUES (2, 'POST', 'http://example.com/api', 'example.com', 201, '[]', '[]', '', '', 'done')", []).unwrap();

        let rows = query_db_table(&db, "history", 0, 10, Some("id"), Some(true), None).unwrap();
        assert_eq!(rows.total, 2);
        assert_eq!(rows.rows.len(), 2);
        assert!(rows.rowids.is_some());

        let search_rows = query_db_table(&db, "history", 0, 10, None, None, Some("POST")).unwrap();
        assert_eq!(search_rows.total, 1);
        assert_eq!(search_rows.rows.len(), 1);

        let rowid = rows.rowids.unwrap()[0];
        let single_row = get_db_row(&db, "history", rowid).unwrap();
        assert!(!single_row.is_empty());

        let res = run_db_query(&db, "SELECT method, status_code FROM history ORDER BY id ASC").unwrap();
        assert_eq!(res.columns, vec!["method", "status_code"]);
        assert_eq!(res.rows.len(), 2);
        assert!(!res.truncated);

        assert!(run_db_query(&db, "DELETE FROM history").is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
