use std::collections::HashMap;
use std::path::PathBuf;

use reqwest::cookie::{CookieStore, Jar};
use rusqlite::{params, Connection};

/// Requests that run around a request on every send, in order.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct RequestSteps {
    pub pre: Vec<String>,
    pub post: Vec<String>,
}

impl RequestSteps {
    fn push(&mut self, phase: &str, step_id: String) {
        match phase {
            "pre" => self.pre.push(step_id),
            _ => self.post.push(step_id),
        }
    }
}

pub fn load_steps(conn: &Connection, request_id: &str) -> rusqlite::Result<RequestSteps> {
    let mut stmt = conn.prepare(
        "SELECT phase, step_request_id FROM request_steps WHERE request_id = ? ORDER BY position",
    )?;
    let mut steps = RequestSteps::default();
    for row in stmt.query_map(params![request_id], |row| Ok((row.get::<_, String>(0)?, row.get(1)?)))? {
        let (phase, step_id) = row?;
        steps.push(&phase, step_id);
    }
    Ok(steps)
}

/// Steps of every request in a workspace, keyed by request id.
pub fn load_workspace_steps(conn: &Connection, workspace_id: &str) -> rusqlite::Result<HashMap<String, RequestSteps>> {
    let mut stmt = conn.prepare(
        "SELECT s.request_id, s.phase, s.step_request_id
         FROM request_steps s
         JOIN requests r ON r.id = s.request_id
         JOIN collections c ON c.id = r.collection_id
         WHERE c.workspace_id = ?
         ORDER BY s.position",
    )?;
    let mut by_request: HashMap<String, RequestSteps> = HashMap::new();
    let rows = stmt.query_map(params![workspace_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get(2)?))
    })?;
    for row in rows {
        let (request_id, phase, step_id) = row?;
        by_request.entry(request_id).or_default().push(&phase, step_id);
    }
    Ok(by_request)
}

/// Replaces a request's steps. Ids of missing requests and of the request itself are dropped.
pub fn save_steps(conn: &Connection, request_id: &str, pre: &[String], post: &[String]) -> rusqlite::Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM request_steps WHERE request_id = ?", params![request_id])?;
    for (phase, ids) in [("pre", pre), ("post", post)] {
        for (position, step_id) in ids.iter().enumerate() {
            tx.execute(
                "INSERT INTO request_steps (request_id, phase, position, step_request_id)
                 SELECT ?1, ?2, ?3, id FROM requests WHERE id = ?4 AND id != ?1",
                params![request_id, phase, position as i64, step_id],
            )?;
        }
    }
    tx.commit()
}

/// Gives a copied request the original's steps, pointing at copies where `copies` maps old ids to new ones.
pub fn copy_steps(
    conn: &Connection,
    from_request_id: &str,
    to_request_id: &str,
    copies: &HashMap<String, String>,
) -> rusqlite::Result<RequestSteps> {
    let original = load_steps(conn, from_request_id)?;
    let remap = |ids: &[String]| -> Vec<String> {
        ids.iter().map(|id| copies.get(id).cloned().unwrap_or_else(|| id.clone())).collect()
    };
    save_steps(conn, to_request_id, &remap(&original.pre), &remap(&original.post))?;
    load_steps(conn, to_request_id)
}

pub fn request_steps(db_path: &PathBuf, request_id: &str) -> Result<RequestSteps, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    load_steps(&conn, request_id).map_err(|e| e.to_string())
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

    fn db_with_requests(ids: &[&str]) -> (PathBuf, Connection) {
        let path = std::env::temp_dir().join(format!("request-steps-test-{}.db", uuid::Uuid::new_v4()));
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE requests (id TEXT PRIMARY KEY)").unwrap();
        conn.execute_batch(include_str!("../migrations/20261005_0019_request_steps.sql")).unwrap();
        for id in ids {
            conn.execute("INSERT INTO requests (id) VALUES (?)", params![id]).unwrap();
        }
        (path, conn)
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn saved_steps_keep_order_and_drop_self_and_missing() {
        let (path, conn) = db_with_requests(&["main", "login", "token", "logout"]);
        save_steps(&conn, "main", &ids(&["login", "main", "gone", "token"]), &ids(&["logout"])).unwrap();
        let steps = load_steps(&conn, "main").unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(steps.pre, ["login", "token"]);
        assert_eq!(steps.post, ["logout"]);
    }

    #[test]
    fn deleting_a_request_removes_it_from_steps_and_drops_its_own() {
        let (path, conn) = db_with_requests(&["main", "token", "logout"]);
        save_steps(&conn, "main", &ids(&["token"]), &ids(&["logout"])).unwrap();
        save_steps(&conn, "token", &[], &ids(&["logout"])).unwrap();
        conn.execute("DELETE FROM requests WHERE id = 'token'", []).unwrap();
        let main = load_steps(&conn, "main").unwrap();
        let rows: i64 = conn.query_row("SELECT COUNT(*) FROM request_steps", [], |r| r.get(0)).unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(main.pre.is_empty());
        assert_eq!(main.post, ["logout"]);
        assert_eq!(rows, 1);
    }

    #[test]
    fn copied_steps_point_at_copies_when_there_are_any() {
        let (path, conn) = db_with_requests(&["submit", "token", "logout", "submit-copy", "token-copy"]);
        save_steps(&conn, "submit", &ids(&["token"]), &ids(&["logout"])).unwrap();
        let copies = HashMap::from([("token".to_string(), "token-copy".to_string())]);
        let copied = copy_steps(&conn, "submit", "submit-copy", &copies).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(copied.pre, ["token-copy"]);
        assert_eq!(copied.post, ["logout"]);
    }
}
