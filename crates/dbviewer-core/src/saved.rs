use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SavedQuery {
    /// Name of the connection profile it belongs to.
    pub connection: String,
    pub name: String,
    pub database: String,
    pub sql: String,
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
}
