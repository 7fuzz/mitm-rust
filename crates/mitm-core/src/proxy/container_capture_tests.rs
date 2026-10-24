use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};

use crate::db::Db;
use crate::state::AppState;

struct Harness {
    proxy_addr: std::net::SocketAddr,
    upstream_requests: Arc<tokio::sync::Mutex<Vec<String>>>,
    upstream_addr: std::net::SocketAddr,
    state: Arc<AppState>,
    db: Db,
    dir: std::path::PathBuf,
    _stop: oneshot::Sender<()>,
}

async fn harness() -> Harness {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let dir = std::env::temp_dir().join(format!("mitm-capture-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let db = Db::new(dir.join("test.db"));
    crate::db::init_database(&db, None).unwrap();
    let (history_tx, history_rx) = mpsc::channel(100);
    let (rewrite_tx, _) = mpsc::channel(1);
    crate::db::actor::start_history_actor(db.clone(), history_rx);
    let state = Arc::new(AppState::new(db.clone(), history_tx, rewrite_tx));

    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_addr = upstream.local_addr().unwrap();
    let upstream_requests = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let seen = Arc::clone(&upstream_requests);
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = upstream.accept().await else { return };
            let seen = Arc::clone(&seen);
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                seen.lock().await.push(String::from_utf8_lossy(&buf[..n]).to_string());
                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok").await;
            });
        }
    });

    let ca = Arc::new(crate::ca::get_ca(dir.join("ca")));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();
    let (stop, stop_rx) = oneshot::channel();
    let proxy_state = Arc::clone(&state);
    tokio::spawn(async move {
        let _ = super::start_proxy_server(None, proxy_state, ca, listener, "default".into(), stop_rx).await;
    });

    Harness { proxy_addr, upstream_requests, upstream_addr, state, db, dir, _stop: stop }
}

fn basic(user: &str, password: &str) -> String {
    format!("Proxy-Authorization: Basic {}\r\n", base64::engine::general_purpose::STANDARD.encode(format!("{user}:{password}")))
}

async fn exchange(addr: std::net::SocketAddr, request: String) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut response)).await;
    String::from_utf8_lossy(&response).to_string()
}

fn get(h: &Harness, path: &str, auth: &str) -> String {
    format!("GET http://{0}{path} HTTP/1.1\r\nHost: {0}\r\n{auth}Connection: close\r\n\r\n", h.upstream_addr)
}

async fn stored(h: &Harness, path: &str) -> Option<(String, i64)> {
    for _ in 0..40 {
        let found = h.db.conn().unwrap().query_row(
            "SELECT container_label, run_id FROM history WHERE path = ?1 AND phase = 'response'",
            [path],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );
        if let Ok(found) = found {
            return Some(found);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    None
}

#[tokio::test]
async fn container_credentials_label_the_flow_and_never_reach_the_origin() {
    let h = harness().await;
    let _ = h.state.containers.register("7", "App A · admin");
    let token = h.state.containers.token().to_string();

    let response = exchange(h.proxy_addr, get(&h, "/labelled", &basic("7", &token))).await;
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert_eq!(stored(&h, "/labelled").await, Some(("App A · admin".to_string(), 0)));
    let upstream = h.upstream_requests.lock().await.clone();
    assert_eq!(upstream.len(), 1);
    assert!(!upstream[0].to_lowercase().contains("proxy-authorization"), "{}", upstream[0]);
    let _ = std::fs::remove_dir_all(&h.dir);
}

#[tokio::test]
async fn anonymous_requests_are_stored_unlabelled() {
    let h = harness().await;
    let response = exchange(h.proxy_addr, get(&h, "/anon", "")).await;
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert_eq!(stored(&h, "/anon").await, Some((String::new(), 0)));
    let _ = std::fs::remove_dir_all(&h.dir);
}

#[tokio::test]
async fn wrong_token_is_refused_with_403_never_407_and_nothing_is_forwarded() {
    let h = harness().await;
    let response = exchange(h.proxy_addr, get(&h, "/denied", &basic("7", "nope"))).await;
    assert!(response.starts_with("HTTP/1.1 403"), "{response}");
    assert!(!response.contains("407"));
    assert!(h.upstream_requests.lock().await.is_empty());

    let connect = format!("CONNECT example.test:443 HTTP/1.1\r\nHost: example.test:443\r\n{}\r\n", basic("7", "nope"));
    let response = exchange(h.proxy_addr, connect).await;
    assert!(response.starts_with("HTTP/1.1 403"), "{response}");
    let _ = std::fs::remove_dir_all(&h.dir);
}

#[tokio::test]
async fn connect_with_credentials_and_anonymous_connect_are_both_accepted() {
    let h = harness().await;
    let token = h.state.containers.token().to_string();
    for auth in [basic("7", &token), String::new()] {
        let mut stream = TcpStream::connect(h.proxy_addr).await.unwrap();
        let request = format!("CONNECT example.test:443 HTTP/1.1\r\nHost: example.test:443\r\n{auth}\r\n");
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut buf = [0u8; 64];
        let n = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buf)).await.unwrap().unwrap();
        assert!(String::from_utf8_lossy(&buf[..n]).starts_with("HTTP/1.1 200"));
    }
    let _ = std::fs::remove_dir_all(&h.dir);
}

#[tokio::test]
async fn container_scope_matches_its_containers_alongside_listeners() {
    use crate::proxy::rules::SourceScope;
    let h = harness().await;
    let scope = SourceScope { all: false, ids: Vec::new(), containers: vec!["App A".to_string()] };
    assert!(h.state.source_matches(&scope, "default", "App A").await);
    assert!(!h.state.source_matches(&scope, "default", "App B").await);
    assert!(!h.state.source_matches(&scope, "default", "").await);

    let both = SourceScope { all: false, ids: vec![0], containers: vec!["App A".to_string()] };
    assert!(h.state.source_matches(&both, "default", "App B").await);
    assert!(h.state.source_matches(&SourceScope::default(), "default", "").await);
    let _ = std::fs::remove_dir_all(&h.dir);
}

#[tokio::test]
async fn intercept_scoped_to_one_container_pauses_only_its_flows() {
    use crate::proxy::rules::SourceScope;
    let h = harness().await;
    let token = h.state.containers.token().to_string();
    h.state.proxy_config.write().await.intercept_enabled = true;
    h.state.proxy_config.write().await.intercept_source_scope =
        SourceScope { all: false, ids: Vec::new(), containers: vec!["7".to_string()] };

    let other = exchange(h.proxy_addr, get(&h, "/other", &basic("8", &token))).await;
    assert!(other.starts_with("HTTP/1.1 200"), "{other}");

    let paused = tokio::spawn({
        let addr = h.proxy_addr;
        let request = get(&h, "/scoped", &basic("7", &token));
        async move { exchange(addr, request).await }
    });
    for _ in 0..40 {
        if !h.state.pending_flows.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(h.state.pending_flows.len(), 1);
    paused.abort();
    let _ = std::fs::remove_dir_all(&h.dir);
}
