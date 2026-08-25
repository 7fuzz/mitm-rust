use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use crate::state::HistoryEntry;

pub fn start_history_actor(
    db_path: PathBuf,
    mut rx: mpsc::Receiver<HistoryEntry>,
) {
    tokio::spawn(async move {
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

    if let Ok(mut conn) = rusqlite::Connection::open(db_path) {
        if let Ok(tx) = conn.transaction() {
            for entry in buffer.iter() {
                let req_headers = serde_json::to_string(&entry.request_headers).unwrap_or_else(|_| "[]".to_string());
                let res_headers = serde_json::to_string(&entry.response_headers).unwrap_or_else(|_| "[]".to_string());

                let _ = tx.execute(
                    "INSERT INTO history (uuid, method, url, host, status_code, request_headers, response_headers, request_body, response_body, phase, duration_ms) 
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    rusqlite::params![
                        entry.uuid,
                        entry.method,
                        entry.url,
                        entry.host,
                        entry.status_code,
                        req_headers,
                        res_headers,
                        entry.request_body,
                        entry.response_body,
                        entry.phase,
                        entry.duration_ms,
                    ],
                );
            }
            let _ = tx.commit();
        }
    }
    buffer.clear();
}
