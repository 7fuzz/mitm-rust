use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use crate::state::HistoryEntry;
use crate::db::prune_history_logs_conn;

pub fn start_history_actor(
    db_path: PathBuf,
    mut rx: mpsc::Receiver<HistoryEntry>,
) {
    tauri::async_runtime::spawn(async move {
        let mut buffer: Vec<HistoryEntry> = Vec::with_capacity(50);
        let mut flush_timer = interval(Duration::from_millis(100));

        loop {
            tokio::select! {
                maybe_entry = rx.recv() => {
                    match maybe_entry {
                        Some(entry) => {
                            buffer.push(entry);
                            if buffer.len() >= 50 {
                                flush_history_batch(&db_path, &mut buffer);
                            }
                        }
                        None => {
                            if !buffer.is_empty() {
                                flush_history_batch(&db_path, &mut buffer);
                            }
                            break;
                        }
                    }
                }
                _ = flush_timer.tick() => {
                    if !buffer.is_empty() {
                        flush_history_batch(&db_path, &mut buffer);
                    }
                }
            }
        }
    });
}

fn flush_history_batch(db_path: &PathBuf, buffer: &mut Vec<HistoryEntry>) {
    if buffer.is_empty() {
        return;
    }

    match rusqlite::Connection::open(db_path) {
        Ok(mut conn) => {
            match conn.transaction() {
                Ok(tx) => {
                    for entry in buffer.iter() {
                        let req_headers = serde_json::to_string(&entry.request_headers).unwrap_or_else(|_| "[]".to_string());
                        let res_headers = serde_json::to_string(&entry.response_headers).unwrap_or_else(|_| "[]".to_string());

                        let res = tx.execute(
                            "INSERT OR REPLACE INTO history (id, method, url, host, path, content_type, response_size, status_code, request_headers, response_headers, request_body, response_body, phase, duration_ms) 
                             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                            rusqlite::params![
                                entry.id,
                                entry.method,
                                entry.url,
                                entry.host,
                                entry.path,
                                entry.content_type,
                                entry.response_size,
                                entry.status_code,
                                req_headers,
                                res_headers,
                                entry.request_body,
                                entry.response_body,
                                entry.phase,
                                entry.duration_ms,
                            ],
                        );
                        if let Err(e) = res {
                            eprintln!("[DB Actor] Error inserting history entry (id: {}): {}", entry.id, e);
                        }
                    }
                    if let Err(e) = tx.commit() {
                        eprintln!("[DB Actor] Failed to commit history batch transaction: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("[DB Actor] Failed to begin transaction: {}", e);
                }
            }

            // Check if history rotation limiter is enabled and prune excess rows directly on conn
            let limiter_enabled: bool = conn
                .query_row(
                    "SELECT value FROM app_preferences WHERE key = 'history_limiter_enabled'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .map(|v| v == "true")
                .unwrap_or(true);

            if limiter_enabled {
                let max_rows: u32 = conn
                    .query_row(
                        "SELECT value FROM app_preferences WHERE key = 'history_limiter_max_rows'",
                        [],
                        |r| r.get::<_, String>(0),
                    )
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(500);

                if let Err(e) = prune_history_logs_conn(&conn, max_rows) {
                    eprintln!("[DB Actor] Error pruning history logs: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("[DB Actor] Failed to open SQLite connection to {:?}: {}", db_path, e);
        }
    }
    buffer.clear();
}
