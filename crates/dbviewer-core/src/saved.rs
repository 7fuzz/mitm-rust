use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SavedQuery {
    /// Name of the connection profile it belongs to.
    pub connection: String,
    pub name: String,
    pub database: String,
    pub sql: String,
}

pub struct SavedQueryStore {
    path: PathBuf,
}

impl SavedQueryStore {
    pub fn default_location() -> Self {
        let dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("dbviewer");
        Self { path: dir.join("queries.json") }
    }

    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<Vec<SavedQuery>> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => serde_json::from_str(&text).with_context(|| format!("parsing {}", self.path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e).with_context(|| format!("reading {}", self.path.display())),
        }
    }

    pub fn save(&self, queries: &[SavedQuery]) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&self.path, serde_json::to_string_pretty(queries)?).with_context(|| format!("writing {}", self.path.display()))
    }
}

/// Replaces a query with the same connection and name, otherwise appends.
pub fn upsert(queries: &mut Vec<SavedQuery>, query: SavedQuery) {
    match queries.iter_mut().find(|q| q.connection == query.connection && q.name == query.name) {
        Some(existing) => *existing = query,
        None => queries.push(query),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(name: &str, sql: &str) -> SavedQuery {
        SavedQuery { connection: "c".into(), name: name.into(), database: "d".into(), sql: sql.into() }
    }

    #[test]
    fn upsert_replaces_by_connection_and_name() {
        let mut queries = vec![query("a", "select 1")];
        upsert(&mut queries, query("a", "select 2"));
        upsert(&mut queries, query("b", "select 3"));
        assert_eq!(queries, vec![query("a", "select 2"), query("b", "select 3")]);
    }

    #[test]
    fn roundtrips() {
        let dir = std::env::temp_dir().join(format!("dbviewer-queries-{}", std::process::id()));
        let store = SavedQueryStore::at(dir.join("q.json"));
        assert!(store.load().unwrap().is_empty());
        store.save(&[query("a", "select 1")]).unwrap();
        assert_eq!(store.load().unwrap(), vec![query("a", "select 1")]);
        std::fs::remove_dir_all(dir).ok();
    }
}
