use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_postgres::{Config, NoTls, SimpleQueryMessage};

use crate::engine::Engine;
use crate::foreign::ForeignKey;
use crate::session::{group_columns, QueryResult, TableKind, TableRef, MAX_ROWS, SCHEMA_ROW_LIMIT};

struct Conn {
    client: tokio_postgres::Client,
    task: JoinHandle<()>,
}

impl Drop for Conn {
    fn drop(&mut self) {
        self.task.abort();
    }
}

const MAINTENANCE_DB: &str = "postgres";

/// One connection per database, opened on first use.
pub struct Postgres {
    config: Config,
    read_only: bool,
    /// `None` when the profile's database is empty, meaning every database on the server.
    only_database: Option<String>,
    conns: Mutex<HashMap<String, Arc<Conn>>>,
}

impl Postgres {
    pub async fn connect(host: &str, port: u16, user: &str, password: &str, database: Option<String>, read_only: bool) -> Result<Self> {
        let mut config = Config::new();
        config.host(host).port(port).user(user).connect_timeout(Duration::from_secs(15));
        if !password.is_empty() {
            config.password(password);
        }
        let backend = Self { config, read_only, only_database: database, conns: Mutex::new(HashMap::new()) };
        let first = backend.only_database.clone().unwrap_or_else(|| MAINTENANCE_DB.to_string());
        backend.conn(&first).await?;
        Ok(backend)
    }

    pub async fn databases(&self) -> Result<Vec<String>> {
        if let Some(only) = &self.only_database {
            return Ok(vec![only.clone()]);
        }
        let result = self
            .query(MAINTENANCE_DB, "SELECT datname FROM pg_database WHERE datallowconn AND NOT datistemplate ORDER BY 1")
            .await?;
        Ok(result.rows.into_iter().filter_map(|row| row.into_iter().next().flatten()).collect())
    }

    async fn conn(&self, database: &str) -> Result<Arc<Conn>> {
        let mut conns = self.conns.lock().await;
        if let Some(conn) = conns.get(database).filter(|c| !c.client.is_closed()) {
            return Ok(Arc::clone(conn));
        }
        let mut config = self.config.clone();
        config.dbname(database);
        let (client, connection) = config.connect(NoTls).await.map_err(|e| anyhow!(describe(&e)))?;
        let task = tokio::spawn(async move {
            let _ = connection.await;
        });
        let conn = Arc::new(Conn { client, task });
        if self.read_only {
            conn.client.batch_execute("SET default_transaction_read_only = on").await.map_err(|e| anyhow!(describe(&e)))?;
        }
        conns.insert(database.to_string(), Arc::clone(&conn));
        Ok(conn)
    }

    pub async fn tables(&self, database: &str) -> Result<Vec<TableRef>> {
        let result = self
            .query(
                database,
                "SELECT n.nspname, c.relname, c.relkind::text
                 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE c.relkind IN ('r','p','v','m','f')
                   AND n.nspname NOT IN ('pg_catalog','information_schema')
                   AND n.nspname NOT LIKE 'pg_toast%'
                   AND NOT c.relispartition
                 ORDER BY 1, 2",
            )
            .await?;
        Ok(result
            .rows
            .into_iter()
            .filter_map(|row| {
                let mut cells = row.into_iter();
                let schema = cells.next()??;
                let name = cells.next()??;
                let kind = match cells.next()??.as_str() {
                    "v" => TableKind::View,
                    "m" => TableKind::MaterializedView,
                    "f" => TableKind::Foreign,
                    _ => TableKind::Table,
                };
                Some(TableRef { engine: Engine::Postgres, database: database.to_string(), schema, name, kind })
            })
            .collect())
    }

    pub async fn primary_key(&self, table: &TableRef) -> Result<Vec<String>> {
        let regclass = Engine::Postgres.literal(&table.qualified());
        let result = self
            .query(
                &table.database,
                &format!(
                    "SELECT a.attname FROM pg_index i
                     JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
                     WHERE i.indrelid = {regclass}::regclass AND i.indisprimary
                     ORDER BY array_position(i.indkey::int2[], a.attnum)"
                ),
            )
            .await?;
        Ok(result.rows.into_iter().filter_map(|row| row.into_iter().next().flatten()).collect())
    }

    pub async fn foreign_keys(&self, table: &TableRef) -> Result<Vec<ForeignKey>> {
        let regclass = Engine::Postgres.literal(&table.qualified());
        let result = self
            .query(
                &table.database,
                &format!(
                    "SELECT a.attname, n2.nspname, c2.relname, a2.attname
                     FROM pg_constraint con
                     JOIN pg_attribute a ON a.attrelid = con.conrelid AND a.attnum = con.conkey[1]
                     JOIN pg_class c2 ON c2.oid = con.confrelid
                     JOIN pg_namespace n2 ON n2.oid = c2.relnamespace
                     JOIN pg_attribute a2 ON a2.attrelid = con.confrelid AND a2.attnum = con.confkey[1]
                     WHERE con.contype = 'f' AND con.conrelid = {regclass}::regclass AND array_length(con.conkey, 1) = 1
                     ORDER BY 1"
                ),
            )
            .await?;
        Ok(result
            .rows
            .into_iter()
            .filter_map(|row| {
                let mut cells = row.into_iter();
                let column = cells.next()??;
                let schema = cells.next()??;
                let name = cells.next()??;
                let target_column = cells.next()??;
                let target = TableRef { engine: Engine::Postgres, database: table.database.clone(), schema, name, kind: TableKind::Table };
                Some(ForeignKey { column, target, target_column })
            })
            .collect())
    }

    pub async fn columns(&self, database: &str) -> Result<HashMap<String, Vec<String>>> {
        let result = self
            .query_limited(
                database,
                "SELECT table_name, column_name FROM information_schema.columns
                 WHERE table_schema NOT IN ('pg_catalog', 'information_schema') AND table_schema NOT LIKE 'pg_toast%'
                 ORDER BY table_schema, table_name, ordinal_position",
                SCHEMA_ROW_LIMIT,
            )
            .await?;
        Ok(group_columns(result.rows))
    }

    pub async fn enum_options(&self, table: &TableRef) -> Result<HashMap<String, Vec<String>>> {
        let regclass = Engine::Postgres.literal(&table.qualified());
        let result = self
            .query(
                &table.database,
                &format!(
                    "SELECT a.attname, e.enumlabel FROM pg_attribute a
                     JOIN pg_enum e ON e.enumtypid = a.atttypid
                     WHERE a.attrelid = {regclass}::regclass AND a.attnum > 0 AND NOT a.attisdropped
                     ORDER BY a.attnum, e.enumsortorder"
                ),
            )
            .await?;
        let mut options: HashMap<String, Vec<String>> = HashMap::new();
        for row in result.rows {
            if let [Some(column), Some(label)] = &row[..] {
                options.entry(column.clone()).or_default().push(label.clone());
            }
        }
        Ok(options)
    }

    /// Uses the simple query protocol, so every value arrives as text and any type displays.
    /// With several statements, the last result set wins.
    pub async fn query(&self, database: &str, sql: &str) -> Result<QueryResult> {
        self.query_limited(database, sql, MAX_ROWS).await
    }

    pub async fn query_limited(&self, database: &str, sql: &str, max_rows: usize) -> Result<QueryResult> {
        let started = Instant::now();
        let conn = self.conn(database).await?;
        let messages = conn.client.simple_query(sql).await.map_err(|e| anyhow!(describe(&e)))?;
        let mut result = QueryResult::default();
        for message in messages {
            match message {
                SimpleQueryMessage::RowDescription(columns) => {
                    result.columns = columns.iter().map(|c| c.name().to_string()).collect();
                    result.rows.clear();
                    result.truncated = false;
                }
                SimpleQueryMessage::Row(row) => {
                    if result.rows.len() >= max_rows {
                        result.truncated = true;
                        continue;
                    }
                    result.rows.push((0..row.len()).map(|i| row.get(i).map(str::to_string)).collect());
                }
                SimpleQueryMessage::CommandComplete(count) => result.affected = Some(count),
                _ => {}
            }
        }
        result.elapsed = started.elapsed();
        Ok(result)
    }
}

fn describe(error: &tokio_postgres::Error) -> String {
    match error.as_db_error() {
        Some(db) => match db.detail() {
            Some(detail) => format!("{}: {detail}", db.message()),
            None => db.message().to_string(),
        },
        None => std::error::Error::source(error).map(|s| s.to_string()).unwrap_or_else(|| error.to_string()),
    }
}
