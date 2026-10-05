use std::collections::HashSet;
use std::path::PathBuf;

use reqwest::cookie::{CookieStore, Jar};
use rusqlite::{params, Connection, OptionalExtension};

/// Requests to run before `request_id`, farthest first. Stops at a cycle or a deleted link.
pub fn pre_request_chain(db_path: &PathBuf, request_id: &str) -> Result<Vec<String>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut seen = HashSet::from([request_id.to_string()]);
    let mut chain = Vec::new();
    let mut current = request_id.to_string();

    loop {
        let next: Option<String> = conn
            .query_row(
                "SELECT p.id FROM requests r JOIN requests p ON p.id = r.pre_request_id WHERE r.id = ?",
                params![current],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        match next {
            Some(id) if seen.insert(id.clone()) => {
                chain.push(id.clone());
                current = id;
            }
            _ => break,
        }
    }

    chain.reverse();
    Ok(chain)
}

/// Shown as the main request's response when a pre-request never reached the server.
pub fn pre_request_failure(db_path: &PathBuf, pre_id: &str, error: &str) -> String {
    let name: Option<String> = Connection::open(db_path)
        .ok()
        .and_then(|conn| {
            conn.query_row("SELECT name FROM requests WHERE id = ?", params![pre_id], |row| row.get(0))
                .ok()
        });
    format!("Pre-request \"{}\" failed: {}", name.as_deref().unwrap_or(pre_id), error)
}

/// The `Cookie` header the jar holds for `url`, so it can be sent and logged explicitly.
pub fn jar_cookie(jar: &Jar, url: &str) -> Option<String> {
    let url = reqwest::Url::parse(url).ok()?;
    jar.cookies(&url)?.to_str().ok().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db_with_links(links: &[(&str, Option<&str>)]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("pre-request-test-{}.db", uuid::Uuid::new_v4()));
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE requests (id TEXT PRIMARY KEY, pre_request_id TEXT)").unwrap();
        for (id, pre) in links {
            conn.execute("INSERT INTO requests (id, pre_request_id) VALUES (?, ?)", params![id, pre]).unwrap();
        }
        path
    }

    #[test]
    fn chain_runs_farthest_first() {
        let db = db_with_links(&[("a", Some("b")), ("b", Some("c")), ("c", None)]);
        let chain = pre_request_chain(&db, "a").unwrap();
        let _ = std::fs::remove_file(&db);
        assert_eq!(chain, ["c", "b"]);
    }

    #[test]
    fn chain_stops_at_cycles_and_missing_links() {
        let db = db_with_links(&[("a", Some("b")), ("b", Some("a")), ("x", Some("gone"))]);
        let cyclic = pre_request_chain(&db, "a").unwrap();
        let dangling = pre_request_chain(&db, "x").unwrap();
        let _ = std::fs::remove_file(&db);
        assert_eq!(cyclic, ["b"]);
        assert!(dangling.is_empty());
    }
}
