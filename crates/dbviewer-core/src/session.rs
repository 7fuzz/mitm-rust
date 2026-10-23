use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, Result};

use crate::edit::Assignment;
use crate::engine::Engine;
use crate::filter::{default_sort, Condition, Filter, Op, Sort};
use crate::foreign::ForeignKey;
use crate::my::MySql;
use crate::pg::Postgres;
use crate::profile::Profile;
use crate::tunnel::Tunnel;

pub(crate) const MAX_ROWS: usize = 5_000;
pub(crate) const SCHEMA_ROW_LIMIT: usize = 200_000;

/// `(table, column)` rows in column order -> columns per table name.
pub(crate) fn group_columns(rows: Vec<Vec<Option<String>>>) -> HashMap<String, Vec<String>> {
    let mut columns: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        if let [Some(table), Some(column)] = &row[..] {
            let list = columns.entry(table.clone()).or_default();
            if !list.contains(column) {
                list.push(column.clone());
            }
        }
    }
    columns
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableKind {
    Table,
    View,
    MaterializedView,
    Foreign,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableRef {
    pub engine: Engine,
    pub database: String,
    /// Empty for MySQL, where the database is the only namespace.
    pub schema: String,
    pub name: String,
    pub kind: TableKind,
}

impl TableRef {
    pub fn qualified(&self) -> String {
        let engine = self.engine;
        match engine {
            Engine::Postgres => format!("{}.{}", engine.quote_ident(&self.schema), engine.quote_ident(&self.name)),
            Engine::MySql | Engine::MariaDb => format!("{}.{}", engine.quote_ident(&self.database), engine.quote_ident(&self.name)),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    /// Rows past the cap were dropped.
    pub truncated: bool,
    pub affected: Option<u64>,
    pub elapsed: Duration,
}

enum Backend {
    Postgres(Postgres),
    MySql(MySql),
}

/// A server connection, all databases sharing one SSH tunnel.
pub struct Session {
    backend: Backend,
    read_only: bool,
    _tunnel: Option<Tunnel>,
}

impl Session {
    pub async fn connect(profile: &Profile, password: &str) -> Result<Self> {
        let (host, port, tunnel) = match &profile.ssh {
            Some(ssh) => {
                let tunnel = Tunnel::open(ssh, &profile.host, profile.port).await?;
                ("127.0.0.1".to_string(), tunnel.local_port, Some(tunnel))
            }
            None => (profile.host.clone(), profile.port, None),
        };
        let database = Some(profile.database.trim().to_string()).filter(|d| !d.is_empty());
        let backend = match profile.engine {
            Engine::Postgres => Backend::Postgres(Postgres::connect(&host, port, &profile.user, password, database, profile.read_only).await?),
            Engine::MySql | Engine::MariaDb => Backend::MySql(MySql::connect(profile.engine, &host, port, &profile.user, password, database, profile.read_only).await?),
        };
        Ok(Self { backend, read_only: profile.read_only, _tunnel: tunnel })
    }

    pub fn engine(&self) -> Engine {
        match &self.backend {
            Backend::Postgres(_) => Engine::Postgres,
            Backend::MySql(b) => b.engine(),
        }
    }

    pub fn read_only(&self) -> bool {
        self.read_only
    }

    pub async fn databases(&self) -> Result<Vec<String>> {
        match &self.backend {
            Backend::Postgres(b) => b.databases().await,
            Backend::MySql(b) => b.databases().await,
        }
    }

    pub async fn tables(&self, database: &str) -> Result<Vec<TableRef>> {
        match &self.backend {
            Backend::Postgres(b) => b.tables(database).await,
            Backend::MySql(b) => b.tables(database).await,
        }
    }

    /// Empty when the relation has no primary key (views, keyless tables).
    pub async fn primary_key(&self, table: &TableRef) -> Result<Vec<String>> {
        match &self.backend {
            Backend::Postgres(b) => b.primary_key(table).await,
            Backend::MySql(b) => b.primary_key(table).await,
        }
    }

    /// Single-column foreign keys only.
    pub async fn foreign_keys(&self, table: &TableRef) -> Result<Vec<ForeignKey>> {
        match &self.backend {
            Backend::Postgres(b) => b.foreign_keys(table).await,
            Backend::MySql(b) => b.foreign_keys(table).await,
        }
    }

    /// Column names per table name for autocomplete.
    pub async fn columns(&self, database: &str) -> Result<HashMap<String, Vec<String>>> {
        match &self.backend {
            Backend::Postgres(b) => b.columns(database).await,
            Backend::MySql(b) => b.columns(database).await,
        }
    }

    /// Allowed values per enum column; empty when the table has none.
    pub async fn enum_options(&self, table: &TableRef) -> Result<HashMap<String, Vec<String>>> {
        match &self.backend {
            Backend::Postgres(b) => b.enum_options(table).await,
            Backend::MySql(b) => b.enum_options(table).await,
        }
    }

    /// Rows of the referenced table, matching `search` in any column when it isn't empty.
    pub async fn lookup(&self, fk: &ForeignKey, search: &str, limit: usize) -> Result<QueryResult> {
        let mut filter = Filter::default();
        if !search.is_empty() {
            filter.conditions.push(Condition { column: None, op: Op::Contains, value: search.to_string() });
        }
        self.browse(&fk.target, limit, 0, &filter, &[]).await
    }

    /// The referenced row for one key value, to show its name next to the id.
    pub async fn lookup_value(&self, fk: &ForeignKey, value: &str) -> Result<QueryResult> {
        let filter = Filter {
            conditions: vec![Condition { column: Some(fk.target_column.clone()), op: Op::Equals, value: value.to_string() }],
            sort: Some(Sort { column: fk.target_column.clone(), descending: false }),
            ..Filter::default()
        };
        self.browse(&fk.target, 1, 0, &filter, &[]).await
    }

    /// Fails when no row matched, so a stale dialog can't silently do nothing.
    pub async fn update_row(&self, table: &TableRef, changes: &[Assignment], key: &[Assignment]) -> Result<()> {
        let result = self.query(&table.database, &crate::edit::update_sql(table, changes, key)).await?;
        match result.affected {
            Some(n) if n > 0 => Ok(()),
            _ => Err(anyhow!("no row matched; it may have been changed or deleted")),
        }
    }

    /// `columns` backs "any column" conditions and the default sort; they're looked up when empty.
    pub async fn browse(&self, table: &TableRef, limit: usize, offset: usize, filter: &Filter, columns: &[String]) -> Result<QueryResult> {
        let mut filter = filter.clone();
        let mut columns = columns.to_vec();
        let any_column = filter.conditions.iter().any(|c| c.column.is_none() && c.is_active());
        if columns.is_empty() && (filter.sort.is_none() || any_column) {
            let probe = format!("SELECT * FROM {} LIMIT 0", table.qualified());
            columns = self.query(&table.database, &probe).await?.columns;
        }
        if filter.sort.is_none() {
            filter.sort = default_sort(&columns);
        }
        let sql = format!("SELECT * FROM {}{} LIMIT {limit} OFFSET {offset}", table.qualified(), filter.sql_suffix(table.engine, &columns));
        self.query(&table.database, &sql).await
    }

    pub async fn query(&self, database: &str, sql: &str) -> Result<QueryResult> {
        match &self.backend {
            Backend::Postgres(b) => b.query(database, sql).await,
            Backend::MySql(b) => b.query(database, sql).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualifies_tables_per_engine() {
        let mut t = TableRef { engine: Engine::Postgres, database: "d".into(), schema: "public".into(), name: "a b".into(), kind: TableKind::Table };
        assert_eq!(t.qualified(), "\"public\".\"a b\"");
        t.engine = Engine::MySql;
        assert_eq!(t.qualified(), "`d`.`a b`");
    }
}
