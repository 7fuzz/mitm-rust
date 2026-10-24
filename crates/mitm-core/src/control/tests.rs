use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc;

use crate::db::Db;
use crate::state::AppState;
use crate::{EventBus, MitmCore};

struct Client {
    lines: tokio::io::Lines<BufReader<tokio::net::unix::OwnedReadHalf>>,
    write: tokio::net::unix::OwnedWriteHalf,
    next_id: i64,
}

impl Client {
    async fn connect(path: &std::path::Path) -> Self {
        let (read, write) = UnixStream::connect(path).await.unwrap().into_split();
        Self { lines: BufReader::new(read).lines(), write, next_id: 0 }
    }

    async fn raw(&mut self, line: &str) -> Value {
        self.write.write_all(format!("{line}\n").as_bytes()).await.unwrap();
        let reply = tokio::time::timeout(Duration::from_secs(5), self.lines.next_line()).await.unwrap().unwrap().unwrap();
        serde_json::from_str(&reply).unwrap()
    }

    async fn call(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let reply = self.raw(&json!({ "id": self.next_id, "method": method, "params": params }).to_string()).await;
        assert_eq!(reply["id"], self.next_id);
        reply
    }
}

async fn core_with_server() -> (MitmCore, Client, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("mitm-control-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let db = Db::new(dir.join("test.db"));
    crate::db::init_database(&db, None).unwrap();
    let (history_tx, _) = mpsc::channel(1);
    let (rewrite_tx, _) = mpsc::channel(1);
    let state = Arc::new(AppState::new(db.clone(), history_tx, rewrite_tx));
    let core = MitmCore { state, event_bus: EventBus::default(), db };
    let socket = dir.join("run").join("control.sock");
    core.start_control_server_at(socket.clone()).await.unwrap();
    let client = Client::connect(&socket).await;
    (core, client, dir)
}

fn insert_flow(core: &MitmCore, id: i64, label: &str, run_id: i64) {
    core.db
        .conn()
        .unwrap()
        .execute(
            "INSERT INTO history (id, method, url, host, status_code, request_headers, response_headers, request_body, response_body, phase, container_label, run_id)
             VALUES (?1, 'GET', 'http://a.test/', 'a.test', 200, '[]', '[]', '', '', 'response', ?2, ?3)",
            rusqlite::params![id, label, run_id],
        )
        .unwrap();
}

#[tokio::test]
async fn socket_is_private_and_status_reports_the_token() {
    let (core, mut client, dir) = core_with_server().await;
    let socket = dir.join("run").join("control.sock");
    assert_eq!(std::fs::metadata(&socket).unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(std::fs::metadata(socket.parent().unwrap()).unwrap().permissions().mode() & 0o777, 0o700);

    let status = client.call("status", Value::Null).await;
    assert_eq!(status["result"]["token"], core.container_proxy_token());
    assert_eq!(status["result"]["listeners"][0]["label"], "default");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn containers_register_rename_run_and_delete_flows() {
    let (core, mut client, dir) = core_with_server().await;
    let mut events = core.event_bus.subscribe();

    assert_eq!(client.call("register_container", json!({ "id": "7", "name": "App A" })).await["result"], json!({}));
    assert_eq!(client.call("start_run", json!({ "id": "7" })).await["result"]["run_id"], 1);
    insert_flow(&core, 1, "App A", 1);
    assert_eq!(client.call("start_run", json!({ "id": "7" })).await["result"]["run_id"], 2);
    insert_flow(&core, 2, "App A", 2);
    insert_flow(&core, 3, "Other", 0);

    client.call("register_container", json!({ "id": "7", "name": "App A · admin" })).await;
    let listed = client.call("list_containers", Value::Null).await;
    assert_eq!(listed["result"], json!([{ "id": "7", "name": "App A · admin", "run_id": 2, "flows": 2 }]));

    assert_eq!(client.call("delete_flows", json!({ "id": "7", "run_id": 1 })).await["result"]["deleted"], 1);
    assert_eq!(client.call("delete_flows", json!({ "id": "7" })).await["result"]["deleted"], 1);
    assert_eq!(crate::db::get_history_count(&core.db, None, false).unwrap(), 1);
    assert!(matches!(events.recv().await.unwrap(), crate::CoreEvent::ContainerFlowsDeleted));

    client.call("remove_container", json!({ "id": "7" })).await;
    assert_eq!(client.call("list_containers", Value::Null).await["result"], json!([]));
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn bad_requests_get_errors_and_the_connection_stays_usable() {
    let (_core, mut client, dir) = core_with_server().await;
    assert!(client.raw("not json").await["error"].as_str().unwrap().contains("Invalid JSON"));
    assert!(client.call("nope", Value::Null).await["error"].as_str().unwrap().contains("Unknown method"));
    assert!(client.call("start_run", json!({})).await["error"].as_str().unwrap().contains("Missing 'id'"));
    assert!(client.call("register_container", json!({ "id": "7", "name": " " })).await["error"].is_string());
    assert!(client.call("status", Value::Null).await["result"].is_object());
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn restarting_the_server_replaces_a_stale_socket() {
    let (core, _client, dir) = core_with_server().await;
    let socket = dir.join("run").join("control.sock");
    core.start_control_server_at(socket.clone()).await.unwrap();
    let mut again = Client::connect(&socket).await;
    assert!(again.call("status", Value::Null).await["result"].is_object());
    core.stop_control_server().await;
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn ca_info_and_pem_come_from_the_current_ca() {
    let data_home = std::env::temp_dir().join(format!("mitm-control-ca-{}", uuid::Uuid::new_v4()));
    std::env::set_var("XDG_DATA_HOME", &data_home);
    let (core, mut client, dir) = core_with_server().await;

    let info = client.call("ca_info", Value::Null).await["result"].clone();
    let pem = client.call("ca_pem", Value::Null).await["result"]["pem"].clone();
    assert!(pem.as_str().unwrap().contains("BEGIN CERTIFICATE"));
    assert_eq!(info["pem"], pem);
    assert_eq!(info["fingerprint"], core.ca_info().unwrap().fingerprint);
    assert_eq!(info["scope"], "global");
    assert_eq!(info["domains"], json!([]));
    assert!(data_home.join("mitm").join("ca").join("ca.crt").exists());
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_home);
}
