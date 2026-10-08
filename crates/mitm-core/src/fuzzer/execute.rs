use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use futures_util::stream::{self, StreamExt};
use rusqlite::Connection;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{apply_payloads, evaluate_matches, insert_results, FuzzConfig, FuzzResult, FuzzTemplate};
use crate::events::{CoreEvent, EventBus};
use crate::repeater::execute::send_tab_request_with;
use crate::repeater::RepeaterTab;

const WRITE_BATCH: usize = 500;

fn template_to_tab(idx: u32, t: &FuzzTemplate) -> RepeaterTab {
    RepeaterTab {
        id: format!("fuzz-{}", idx),
        method: t.method.clone(),
        url: t.url.clone(),
        headers: t.headers.clone(),
        params: t.params.clone(),
        body_type: t.body_type.clone(),
        body_content: t.body.clone(),
        extract_rules: Vec::new(),
        pre_requests: vec![],
        post_requests: vec![],
        order_index: 0,
        created_at_ms: 0,
        updated_at_ms: 0,
        execution_count: 0,
        last_status_code: None,
        last_duration_ms: None,
    }
}

async fn run_one(client: &reqwest::Client, idx: u32, config: &FuzzConfig, names: &[String], payloads: Vec<String>) -> FuzzResult {
    let values: HashMap<String, String> = names.iter().cloned().zip(payloads.iter().cloned()).collect();
    let applied = apply_payloads(&config.template, &values);
    let tab = template_to_tab(idx, &applied);
    match send_tab_request_with(client, &tab).await {
        Ok(sent) => FuzzResult {
            idx,
            payloads,
            matches: evaluate_matches(&config.match_rules, &sent.response_body),
            method: tab.method,
            url: sent.final_url,
            request_headers: sent.request_headers,
            request_body: sent.request_body,
            status_code: sent.status_code,
            response_headers: sent.response_headers,
            response_size: sent.response_size,
            duration_ms: sent.duration_ms,
            error: (sent.status_code == 0).then_some(sent.status_text),
            response_body: Some(sent.response_body),
        },
        Err(e) => FuzzResult {
            idx,
            payloads,
            method: applied.method,
            url: applied.url,
            request_headers: applied.headers,
            request_body: applied.body,
            status_code: 0,
            response_headers: Vec::new(),
            response_body: None,
            response_size: 0,
            duration_ms: 0,
            error: Some(e),
            matches: vec![String::new(); config.match_rules.len()],
        },
    }
}

/// Sends every combo with the configured concurrency until done or `cancel` fires. Cancelling drops the
/// in-flight requests, so nothing sent before the stop lands after it except results already being written.
pub(crate) async fn run_attack(
    bus: EventBus,
    db_path: PathBuf,
    run_id: i64,
    config: FuzzConfig,
    names: Vec<String>,
    combos: Vec<Vec<String>>,
    cancel: CancellationToken,
) {
    let total = combos.len();
    let concurrency = config.concurrency.clamp(1, 200) as usize;
    let delay = Duration::from_millis(config.delay_ms);
    let (tx, rx) = mpsc::channel::<FuzzResult>(WRITE_BATCH * 2);
    let writer = tokio::task::spawn_blocking({
        let (bus, db_path) = (bus.clone(), db_path.clone());
        move || write_results(&bus, &db_path, run_id, rx)
    });

    let completed = match crate::repeater::execute::http_client(None) {
        Ok(client) => {
            let (config, names) = (&config, &names);
            let client = &client;
            let mut stream = stream::iter(combos.into_iter().enumerate())
                .map(|(i, payloads)| async move {
                    if !delay.is_zero() {
                        tokio::time::sleep(delay).await;
                    }
                    run_one(client, i as u32, config, names, payloads).await
                })
                .buffer_unordered(concurrency);
            let mut completed = 0;
            loop {
                let result = tokio::select! {
                    biased;
                    _ = cancel.cancelled() => break,
                    next = stream.next() => match next {
                        Some(result) => result,
                        None => break,
                    },
                };
                completed += 1;
                if tx.send(result).await.is_err() {
                    break;
                }
            }
            completed
        }
        Err(e) => {
            eprintln!("[fuzzer] {e}");
            0
        }
    };

    drop(tx);
    let _ = writer.await;
    let stopped = cancel.is_cancelled();
    // Cancelling on a normal finish too is what makes `FuzzControl::is_running` false.
    cancel.cancel();
    bus.emit(CoreEvent::FuzzFinished { run_id, completed, total, stopped });
}

/// Rows are announced only after they're committed, so a row the UI shows can always be opened.
fn write_results(bus: &EventBus, db_path: &PathBuf, run_id: i64, mut rx: mpsc::Receiver<FuzzResult>) {
    let mut conn = Connection::open(db_path).ok();
    if let Some(conn) = &conn {
        let _ = conn.busy_timeout(Duration::from_secs(5));
    }
    while let Some(first) = rx.blocking_recv() {
        let mut batch = vec![first];
        while batch.len() < WRITE_BATCH {
            match rx.try_recv() {
                Ok(result) => batch.push(result),
                Err(_) => break,
            }
        }
        if let Some(conn) = conn.as_mut() {
            if let Err(e) = insert_results(conn, run_id, &batch) {
                eprintln!("[fuzzer] saving results failed: {e}");
            }
        }
        for result in &batch {
            bus.emit(CoreEvent::FuzzResult { run_id, row: result.row() });
        }
    }
}

/// Stops the current attack, if one is running.
#[derive(Default)]
pub struct FuzzControl {
    cancel: std::sync::Mutex<Option<CancellationToken>>,
}

impl FuzzControl {
    pub(crate) fn begin(&self) -> Result<CancellationToken, String> {
        let mut slot = self.cancel.lock().unwrap_or_else(|e| e.into_inner());
        if slot.as_ref().is_some_and(|token| !token.is_cancelled()) {
            return Err("A fuzz attack is already running.".to_string());
        }
        let token = CancellationToken::new();
        *slot = Some(token.clone());
        Ok(token)
    }

    pub fn stop(&self) {
        if let Some(token) = self.cancel.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            token.cancel();
        }
    }

    pub fn is_running(&self) -> bool {
        self.cancel.lock().unwrap_or_else(|e| e.into_inner()).as_ref().is_some_and(|t| !t.is_cancelled())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fuzzer::{generate_combos, FuzzVariable, PayloadSet};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn temp_db() -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("mitm-fuzz-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("test.db");
        crate::db::init_database(&db, None).unwrap();
        (db, dir)
    }

    async fn server(answer: bool) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else { return };
                tokio::spawn(async move {
                    let mut buf = [0u8; 4096];
                    let _ = socket.read(&mut buf).await;
                    if answer {
                        let _ = socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\nhello abc").await;
                    } else {
                        tokio::time::sleep(Duration::from_secs(60)).await;
                    }
                });
            }
        });
        format!("http://{addr}/{{{{n}}}}")
    }

    fn config(url: String, count: usize) -> FuzzConfig {
        FuzzConfig {
            template: FuzzTemplate {
                method: "GET".into(),
                url,
                headers: vec![],
                params: vec![],
                body_type: "none".into(),
                body: None,
            },
            attack_type: "sniper".into(),
            variables: vec![FuzzVariable {
                name: "n".into(),
                set: PayloadSet {
                    kind: "list".into(),
                    list: (0..count).map(|i| i.to_string()).collect(),
                    from: 0.0,
                    to: 0.0,
                    step: 1.0,
                    pad: 0,
                    url_encode: false,
                    prefix: String::new(),
                    suffix: String::new(),
                },
            }],
            match_rules: vec![crate::fuzzer::MatchRule { name: "abc".into(), kind: "contains".into(), pattern: "abc".into() }],
            concurrency: 4,
            delay_ms: 0,
        }
    }

    async fn run(db: &PathBuf, config: FuzzConfig, stop_after: Option<Duration>) -> Vec<CoreEvent> {
        let bus = EventBus::new(1024);
        let mut events = bus.subscribe();
        let (names, combos) = generate_combos(&config).unwrap();
        crate::fuzzer::begin_run_db(db, 1, &config, combos.len()).unwrap();
        let cancel = CancellationToken::new();
        let attack = tokio::spawn(run_attack(bus, db.clone(), 1, config, names, combos, cancel.clone()));
        if let Some(after) = stop_after {
            tokio::time::sleep(after).await;
            cancel.cancel();
        }
        tokio::time::timeout(Duration::from_secs(5), attack).await.unwrap().unwrap();
        let mut out = Vec::new();
        while let Ok(event) = events.try_recv() {
            out.push(event);
        }
        out
    }

    #[tokio::test]
    async fn stores_every_result_before_announcing_it() {
        let (db, dir) = temp_db();
        let events = run(&db, config(server(true).await, 6), None).await;

        let rows: Vec<_> = events.iter().filter(|e| matches!(e, CoreEvent::FuzzResult { .. })).collect();
        assert_eq!(rows.len(), 6);
        assert!(matches!(events.last(), Some(CoreEvent::FuzzFinished { completed: 6, total: 6, stopped: false, .. })));

        let stored = crate::fuzzer::get_run_rows_db(&db, 1).unwrap();
        assert_eq!(stored.len(), 6);
        assert_eq!((stored[0].status_code, stored[0].matches.clone()), (200, vec!["1".to_string()]));
        let full = crate::fuzzer::get_result_db(&db, 1, 3).unwrap().unwrap();
        assert_eq!(full.response_body.as_deref(), Some("hello abc"));
        assert_eq!(crate::fuzzer::search_responses_db(&db, 1, "HELLO").unwrap().len(), 6);
        assert!(crate::fuzzer::list_runs_db(&db).unwrap().is_empty());

        crate::fuzzer::discard_unsaved_runs_db(&db).unwrap();
        assert!(crate::fuzzer::get_run_rows_db(&db, 1).unwrap().is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn stop_drops_in_flight_requests() {
        let (db, dir) = temp_db();
        let events = run(&db, config(server(false).await, 20), Some(Duration::from_millis(200))).await;
        assert!(!events.iter().any(|e| matches!(e, CoreEvent::FuzzResult { .. })));
        assert!(matches!(events.last(), Some(CoreEvent::FuzzFinished { completed: 0, total: 20, stopped: true, .. })));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
