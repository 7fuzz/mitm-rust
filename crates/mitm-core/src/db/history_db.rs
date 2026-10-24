use crate::proxy::history::{HistoryDetail, HistoryEntry};
use crate::db::Db;

pub fn get_history_logs(
    db: &Db,
    limit: u32,
    offset: u32,
    search_term: Option<&str>,
    method_filter: Option<&str>,
    status_range: Option<&str>,
    include_bodies: bool,
) -> Result<Vec<HistoryEntry>, String> {
    let conn = db.conn().map_err(|e| e.to_string())?;

    let select_fields = if include_bodies { FULL_FIELDS } else { SUMMARY_FIELDS };

    let mut query = format!("SELECT {} FROM history WHERE 1=1", select_fields);
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(search) = search_term {
        if !search.trim().is_empty() {
            query.push_str(" AND (url LIKE ? OR host LIKE ? OR method LIKE ?)");
            let pattern = format!("%{}%", search.trim());
            params.push(Box::new(pattern.clone()));
            params.push(Box::new(pattern.clone()));
            params.push(Box::new(pattern));
        }
    }

    if let Some(method) = method_filter {
        if !method.trim().is_empty() && method.to_uppercase() != "ALL" {
            query.push_str(" AND method = ?");
            params.push(Box::new(method.to_uppercase()));
        }
    }

    if let Some(range) = status_range {
        match range {
            "2xx" => query.push_str(" AND (status_code >= 200 AND status_code < 300)"),
            "3xx" => query.push_str(" AND (status_code >= 300 AND status_code < 400)"),
            "4xx" => query.push_str(" AND (status_code >= 400 AND status_code < 500)"),
            "5xx" => query.push_str(" AND (status_code >= 500)"),
            _ => {}
        }
    }

    query.push_str(" ORDER BY id DESC LIMIT ? OFFSET ?");
    params.push(Box::new(limit));
    params.push(Box::new(offset));

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    if include_bodies {
        let rows: Vec<_> = stmt
            .query_map(&param_refs[..], full_from_row)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        rows.into_iter().map(|row| with_bodies(&conn, row)).collect()
    } else {
        let entries = stmt
            .query_map(&param_refs[..], summary_from_row)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }
}

const SUMMARY_FIELDS: &str = "id, method, url, host, path, content_type, response_size, status_code, phase, duration_ms, created_at, COALESCE(is_intercepted, 0), COALESCE(is_rewritten, 0), COALESCE(is_failed, 0), COALESCE(listener_label, ''), request_at, response_at, container_label, run_id";

const FULL_FIELDS: &str = "id, method, url, host, path, content_type, response_size, status_code, request_headers, response_headers, request_body_hash, response_body_hash, phase, duration_ms, created_at, COALESCE(is_intercepted, 0), COALESCE(is_rewritten, 0), COALESCE(is_failed, 0), COALESCE(listener_label, ''), request_at, response_at, container_label, run_id";

type BodyHashes = (Option<Vec<u8>>, Option<Vec<u8>>);

/// A full row with its body hashes; `with_bodies` reads the bodies they point to.
fn full_from_row(row: &rusqlite::Row) -> rusqlite::Result<(HistoryEntry, BodyHashes)> {
    let headers = |idx: usize| -> Vec<(String, String)> {
        let json: String = row.get(idx).unwrap_or_else(|_| "[]".to_string());
        serde_json::from_str(&json).unwrap_or_default()
    };
    let entry = HistoryEntry {
        id: row.get(0)?,
        method: row.get(1)?,
        url: row.get(2)?,
        host: row.get(3)?,
        path: row.get(4)?,
        content_type: row.get(5)?,
        response_size: row.get(6)?,
        status_code: row.get(7)?,
        request_headers: headers(8),
        response_headers: headers(9),
        request_body: String::new(),
        response_body: String::new(),
        phase: row.get(12)?,
        duration_ms: row.get(13)?,
        created_at: row.get(14)?,
        is_intercepted: row.get::<_, i32>(15)? != 0,
        is_rewritten: row.get::<_, i32>(16)? != 0,
        is_failed: row.get::<_, i32>(17)? != 0,
        listener_label: row.get(18)?,
        request_at: row.get(19)?,
        response_at: row.get(20)?,
        container_label: row.get(21)?,
        run_id: row.get(22)?,
    };
    Ok((entry, (row.get(10)?, row.get(11)?)))
}

fn with_bodies(conn: &rusqlite::Connection, (mut entry, (request, response)): (HistoryEntry, BodyHashes)) -> Result<HistoryEntry, String> {
    entry.request_body = super::bodies::get(conn, request.as_deref())?;
    entry.response_body = super::bodies::get(conn, response.as_deref())?;
    Ok(entry)
}

fn summary_from_row(row: &rusqlite::Row) -> rusqlite::Result<HistoryEntry> {
    Ok(HistoryEntry {
        id: row.get(0)?,
        method: row.get(1)?,
        url: row.get(2)?,
        host: row.get(3)?,
        path: row.get(4)?,
        content_type: row.get(5)?,
        response_size: row.get(6)?,
        status_code: row.get(7)?,
        request_headers: Vec::new(),
        response_headers: Vec::new(),
        request_body: String::new(),
        response_body: String::new(),
        phase: row.get(8)?,
        duration_ms: row.get(9)?,
        created_at: row.get(10)?,
        is_intercepted: row.get::<_, i32>(11)? != 0,
        is_rewritten: row.get::<_, i32>(12)? != 0,
        is_failed: row.get::<_, i32>(13)? != 0,
        listener_label: row.get(14)?,
        request_at: row.get(15)?,
        response_at: row.get(16)?,
        container_label: row.get(17)?,
        run_id: row.get(18)?,
    })
}

/// A search over the fields the traffic list shows, optionally the raw header JSON too.
fn search_clause(search: Option<&str>, include_headers: bool) -> (String, Option<String>) {
    let Some(term) = search.map(str::trim).filter(|t| !t.is_empty()) else {
        return (String::new(), None);
    };
    let mut fields = vec!["url", "host", "path", "method", "content_type", "listener_label", "CAST(status_code AS TEXT)"];
    if include_headers {
        fields.extend(["request_headers", "response_headers"]);
    }
    let conditions: Vec<String> = fields.iter().map(|f| format!("{f} LIKE ?1 ESCAPE '\\'")).collect();
    let escaped = term.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    (format!(" AND ({})", conditions.join(" OR ")), Some(format!("%{escaped}%")))
}

/// Newest first; pass the smallest id seen so far as `before_id` for the next page, which stays stable while new flows arrive.
pub fn get_history_page(
    db: &Db,
    search: Option<&str>,
    include_headers: bool,
    before_id: Option<i64>,
    limit: u32,
) -> Result<Vec<HistoryEntry>, String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let (clause, pattern) = search_clause(search, include_headers);
    let sql = format!(
        "SELECT {SUMMARY_FIELDS} FROM history WHERE id < ?2{clause} ORDER BY id DESC LIMIT ?3"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![pattern, before_id.unwrap_or(i64::MAX), limit], summary_from_row)
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_history_count(db: &Db, search: Option<&str>, include_headers: bool) -> Result<u64, String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let (clause, pattern) = search_clause(search, include_headers);
    let sql = format!("SELECT COUNT(*) FROM history WHERE 1=1{clause}");
    let count = match pattern {
        Some(pattern) => conn.query_row(&sql, [pattern], |row| row.get(0)),
        None => conn.query_row(&sql, [], |row| row.get(0)),
    };
    count.map_err(|e| e.to_string())
}

/// Every stored flow without headers or bodies, newest first.
pub fn get_history_summaries(db: &Db) -> Result<Vec<HistoryEntry>, String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(&format!("SELECT {SUMMARY_FIELDS} FROM history ORDER BY id DESC"))
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], summary_from_row).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Full flows with headers and bodies, oldest first.
pub fn get_history_by_ids(db: &Db, ids: &[i64]) -> Result<Vec<HistoryEntry>, String> {
    const CHUNK: usize = 500;
    let conn = db.conn().map_err(|e| e.to_string())?;
    let mut entries = Vec::with_capacity(ids.len());
    for chunk in ids.chunks(CHUNK) {
        let placeholders = vec!["?"; chunk.len()].join(",");
        let mut stmt = conn
            .prepare(&format!("SELECT {FULL_FIELDS} FROM history WHERE id IN ({placeholders})"))
            .map_err(|e| e.to_string())?;
        let rows: Vec<_> = stmt
            .query_map(rusqlite::params_from_iter(chunk), full_from_row)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        for row in rows {
            entries.push(with_bodies(&conn, row)?);
        }
    }
    entries.sort_by_key(|e| e.id);
    Ok(entries)
}

pub fn get_history_detail(
    db: &Db,
    id: i64,
) -> Result<Option<HistoryDetail>, String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, request_headers, response_headers, request_body_hash, response_body_hash FROM history WHERE id = ?")
        .map_err(|e| e.to_string())?;

    let mut rows = stmt
        .query_map([id], |row| {
            let req_headers_json: String = row.get(1)?;
            let res_headers_json: String = row.get(2)?;
            let request_hash: Option<Vec<u8>> = row.get(3)?;
            let response_hash: Option<Vec<u8>> = row.get(4)?;

            let request_headers: Vec<(String, String)> = serde_json::from_str(&req_headers_json).unwrap_or_default();
            let response_headers: Vec<(String, String)> = serde_json::from_str(&res_headers_json).unwrap_or_default();

            let detail = HistoryDetail { id: row.get(0)?, request_headers, response_headers, request_body: String::new(), response_body: String::new() };
            Ok((detail, request_hash, response_hash))
        })
        .map_err(|e| e.to_string())?;

    if let Some(res) = rows.next() {
        let (mut detail, request_hash, response_hash) = res.map_err(|e| e.to_string())?;
        detail.request_body = super::bodies::get(&conn, request_hash.as_deref())?;
        detail.response_body = super::bodies::get(&conn, response_hash.as_deref())?;
        Ok(Some(detail))
    } else {
        Ok(None)
    }
}

pub fn clear_history(db: &Db) -> Result<(), String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    conn.execute_batch(
        "DELETE FROM history;
         DELETE FROM sqlite_sequence WHERE name = 'history';"
    ).map_err(|e| e.to_string())?;
    super::bodies::collect_garbage(&conn)?;
    super::storage::reclaim(&conn)?;
    Ok(())
}

pub fn delete_history_entry(db: &Db, id: i64) -> Result<(), String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM history WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    super::bodies::collect_garbage(&conn)?;
    super::storage::reclaim(&conn)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db_with_flows(count: i64) -> (crate::db::Db, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("mitm-history-page-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = crate::db::Db::new(dir.join("test.db"));
        crate::db::init_database(&db, None).unwrap();
        let conn = crate::db::open(&db).unwrap();
        for id in 1..=count {
            let host = if id % 2 == 0 { "even.test" } else { "odd_test" };
            let headers = if id == 3 { r#"[["X-Trace","needle"]]"# } else { "[]" };
            conn.execute(
                "INSERT INTO history (id, method, url, host, status_code, request_headers, response_headers, request_body, response_body, phase) VALUES (?1, 'GET', 'http://' || ?2, ?2, 200, ?3, '[]', '', '', 'response')",
                rusqlite::params![id, host, headers],
            )
            .unwrap();
        }
        (db, dir)
    }

    #[test]
    fn pages_walk_backwards_by_id_without_overlap() {
        let (db, dir) = db_with_flows(25);
        let first = get_history_page(&db, None, false, None, 10).unwrap();
        assert_eq!(first.first().map(|e| e.id), Some(25));
        let next = get_history_page(&db, None, false, first.last().map(|e| e.id), 10).unwrap();
        assert_eq!(next.first().map(|e| e.id), Some(15));
        assert_eq!(get_history_count(&db, None, false).unwrap(), 25);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn summaries_cover_every_row_and_full_rows_load_by_id() {
        let (db, dir) = db_with_flows(1203);
        assert_eq!(get_history_summaries(&db).unwrap().len(), 1203);

        let ids: Vec<i64> = (1..=1203).rev().filter(|id| id % 2 == 1).collect();
        let full = get_history_by_ids(&db, &ids).unwrap();
        assert_eq!(full.len(), 602);
        assert_eq!(full.first().map(|e| e.id), Some(1));
        assert_eq!(full.iter().find(|e| e.id == 3).unwrap().request_headers, vec![("X-Trace".into(), "needle".into())]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn search_covers_headers_only_when_asked_and_escapes_wildcards() {
        let (db, dir) = db_with_flows(6);
        assert_eq!(get_history_count(&db, Some("needle"), false).unwrap(), 0);
        assert_eq!(get_history_count(&db, Some("needle"), true).unwrap(), 1);
        assert_eq!(get_history_count(&db, Some("even"), false).unwrap(), 3);
        assert_eq!(get_history_count(&db, Some("_"), false).unwrap(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
