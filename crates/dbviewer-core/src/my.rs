use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use mysql_async::prelude::Queryable;
use mysql_async::{Conn, OptsBuilder, Row, Value};
use tokio::sync::Mutex;

use crate::engine::Engine;
use crate::foreign::ForeignKey;
use crate::session::{group_columns, QueryResult, TableKind, TableRef, MAX_ROWS, SCHEMA_ROW_LIMIT};

const SYSTEM_DATABASES: [&str; 4] = ["information_schema", "mysql", "performance_schema", "sys"];

/// One connection per database, opened on first use. The empty key is the server-level
/// connection used to list databases.
pub struct MySql {
    engine: Engine,
    opts: OptsBuilder,
    read_only: bool,
    /// `None` when the profile's database is empty, meaning every database on the server.
    only_database: Option<String>,
    conns: Mutex<HashMap<String, Arc<Mutex<Conn>>>>,
}

impl MySql {
    pub async fn connect(engine: Engine, host: &str, port: u16, user: &str, password: &str, database: Option<String>, read_only: bool) -> Result<Self> {
        let opts = OptsBuilder::default()
            .ip_or_hostname(host)
            .tcp_port(port)
            .user(Some(user))
            .pass((!password.is_empty()).then_some(password))
            .client_found_rows(true)
            .wait_timeout(None);
        let backend = Self { engine, opts, read_only, only_database: database, conns: Mutex::new(HashMap::new()) };
        let first = backend.only_database.clone().unwrap_or_default();
        backend.conn(&first).await?;
        Ok(backend)
    }

    pub fn engine(&self) -> Engine {
        self.engine
    }

    pub async fn databases(&self) -> Result<Vec<String>> {
        if let Some(only) = &self.only_database {
            return Ok(vec![only.clone()]);
        }
        let result = self.query("", "SHOW DATABASES").await?;
        Ok(result
            .rows
            .into_iter()
            .filter_map(|row| row.into_iter().next().flatten())
            .filter(|name| !SYSTEM_DATABASES.contains(&name.to_lowercase().as_str()))
            .collect())
    }

    async fn conn(&self, database: &str) -> Result<Arc<Mutex<Conn>>> {
        let mut conns = self.conns.lock().await;
        if let Some(conn) = conns.get(database) {
            return Ok(Arc::clone(conn));
        }
        let opts = self.opts.clone().db_name((!database.is_empty()).then_some(database));
        let mut conn = tokio::time::timeout(Duration::from_secs(15), Conn::new(opts))
            .await
            .map_err(|_| anyhow!("connection timed out"))?
            .map_err(|e| anyhow!(describe(&e)))?;
        if self.read_only {
            conn.query_drop("SET SESSION TRANSACTION READ ONLY").await.map_err(|e| anyhow!(describe(&e)))?;
        }
        let conn = Arc::new(Mutex::new(conn));
        conns.insert(database.to_string(), Arc::clone(&conn));
        Ok(conn)
    }

    pub async fn tables(&self, database: &str) -> Result<Vec<TableRef>> {
        let sql = format!(
            "SELECT table_name, table_type FROM information_schema.tables WHERE table_schema = {} ORDER BY 1",
            self.engine.literal(database)
        );
        let result = self.query(database, &sql).await?;
        Ok(result
            .rows
            .into_iter()
            .filter_map(|row| {
                let mut cells = row.into_iter();
                let name = cells.next()??;
                let kind = match cells.next()??.to_uppercase().as_str() {
                    "VIEW" | "SYSTEM VIEW" => TableKind::View,
                    _ => TableKind::Table,
                };
                Some(TableRef { engine: self.engine, database: database.to_string(), schema: String::new(), name, kind })
            })
            .collect())
    }

    pub async fn row_estimates(&self, database: &str) -> Result<HashMap<(String, String), i64>> {
        let sql = format!(
            "SELECT table_name, CAST(table_rows AS CHAR) FROM information_schema.tables WHERE table_schema = {} AND table_type = 'BASE TABLE'",
            self.engine.literal(database)
        );
        let result = self.query(database, &sql).await?;
        Ok(result
            .rows
            .into_iter()
            .filter_map(|row| {
                let mut cells = row.into_iter();
                let name = cells.next()??;
                let count = cells.next()??.parse().ok()?;
                Some(((String::new(), name), count))
            })
            .collect())
    }

    pub async fn primary_key(&self, table: &TableRef) -> Result<Vec<String>> {
        let sql = format!(
            "SELECT column_name FROM information_schema.key_column_usage
             WHERE table_schema = {} AND table_name = {} AND constraint_name = 'PRIMARY'
             ORDER BY ordinal_position",
            self.engine.literal(&table.database),
            self.engine.literal(&table.name)
        );
        let result = self.query(&table.database, &sql).await?;
        Ok(result.rows.into_iter().filter_map(|row| row.into_iter().next().flatten()).collect())
    }

    pub async fn foreign_keys(&self, table: &TableRef) -> Result<Vec<ForeignKey>> {
        let sql = format!(
            "SELECT constraint_name, column_name, referenced_table_schema, referenced_table_name, referenced_column_name
             FROM information_schema.key_column_usage
             WHERE table_schema = {} AND table_name = {} AND referenced_table_name IS NOT NULL
             ORDER BY constraint_name, ordinal_position",
            self.engine.literal(&table.database),
            self.engine.literal(&table.name)
        );
        let result = self.query(&table.database, &sql).await?;
        let rows: Vec<Vec<String>> = result.rows.into_iter().filter_map(|row| row.into_iter().collect::<Option<Vec<_>>>()).collect();
        Ok(rows
            .iter()
            .filter(|row| rows.iter().filter(|other| other[0] == row[0]).count() == 1)
            .map(|row| ForeignKey {
                column: row[1].clone(),
                target: TableRef { engine: self.engine, database: row[2].clone(), schema: String::new(), name: row[3].clone(), kind: TableKind::Table },
                target_column: row[4].clone(),
            })
            .collect())
    }

    pub async fn columns(&self, database: &str) -> Result<HashMap<String, Vec<String>>> {
        let sql = format!(
            "SELECT table_name, column_name FROM information_schema.columns WHERE table_schema = {} ORDER BY table_name, ordinal_position",
            self.engine.literal(database)
        );
        let result = self.query_limited(database, &sql, SCHEMA_ROW_LIMIT).await?;
        Ok(group_columns(result.rows))
    }

    pub async fn enum_options(&self, table: &TableRef) -> Result<HashMap<String, Vec<String>>> {
        let sql = format!(
            "SELECT column_name, column_type FROM information_schema.columns
             WHERE table_schema = {} AND table_name = {} AND data_type = 'enum'",
            self.engine.literal(&table.database),
            self.engine.literal(&table.name)
        );
        let result = self.query(&table.database, &sql).await?;
        Ok(result
            .rows
            .into_iter()
            .filter_map(|row| match &row[..] {
                [Some(column), Some(definition)] => Some((column.clone(), parse_enum_values(definition))),
                _ => None,
            })
            .collect())
    }

    /// With several statements, the last result set wins. A dead connection is dropped so the
    /// next call opens a fresh one.
    pub async fn query(&self, database: &str, sql: &str) -> Result<QueryResult> {
        self.query_limited(database, sql, MAX_ROWS).await
    }

    pub async fn query_limited(&self, database: &str, sql: &str, max_rows: usize) -> Result<QueryResult> {
        let started = Instant::now();
        let conn = self.conn(database).await?;
        let outcome = run(&mut *conn.lock().await, sql, max_rows).await;
        let mut result = match outcome {
            Ok(result) => result,
            Err(e) => {
                if matches!(e, mysql_async::Error::Io(_) | mysql_async::Error::Driver(_)) {
                    self.conns.lock().await.remove(database);
                }
                return Err(anyhow!(describe(&e)));
            }
        };
        result.elapsed = started.elapsed();
        Ok(result)
    }
}

async fn run(conn: &mut Conn, sql: &str, max_rows: usize) -> std::result::Result<QueryResult, mysql_async::Error> {
    let mut stream = conn.query_iter(sql).await?;
    let mut result = QueryResult::default();
    loop {
        let columns: Vec<String> = stream.columns_ref().iter().map(|c| c.name_str().to_string()).collect();
        let has_rows = !columns.is_empty();
        if !has_rows {
            result.affected = Some(stream.affected_rows());
        }
        let (rows, truncated) = stream
            .reduce((Vec::new(), false), |(mut rows, mut truncated): (Vec<Vec<Option<String>>>, bool), row: Row| {
                if rows.len() >= max_rows {
                    truncated = true;
                } else {
                    rows.push(row.unwrap().iter().map(cell).collect());
                }
                (rows, truncated)
            })
            .await?;
        if has_rows {
            result.columns = columns;
            result.rows = rows;
            result.truncated = truncated;
        }
        if stream.is_empty() {
            break;
        }
    }
    Ok(result)
}

/// `enum('a','it''s','c')` -> `a`, `it's`, `c`.
fn parse_enum_values(definition: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = definition.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, in_quotes) {
            ('\'', false) => in_quotes = true,
            ('\'', true) if chars.peek() == Some(&'\'') => {
                current.push('\'');
                chars.next();
            }
            ('\'', true) => {
                in_quotes = false;
                values.push(std::mem::take(&mut current));
            }
            ('\\', true) => current.extend(chars.next()),
            (c, true) => current.push(c),
            _ => {}
        }
    }
    values
}

fn cell(value: &Value) -> Option<String> {
    match value {
        Value::NULL => None,
        Value::Bytes(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        other => Some(other.as_sql(true).trim_matches('\'').to_string()),
    }
}

fn describe(error: &mysql_async::Error) -> String {
    match error {
        mysql_async::Error::Server(server) => server.message.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_enum_definitions() {
        assert_eq!(parse_enum_values("enum('a','it''s','c d')"), vec!["a", "it's", "c d"]);
        assert_eq!(parse_enum_values("enum('x,y','z')"), vec!["x,y", "z"]);
    }
}
