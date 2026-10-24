mod api;
mod handlers;

#[cfg(test)]
mod tests;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::oneshot;

use crate::MitmCore;

pub fn default_socket_path() -> PathBuf {
    dirs::runtime_dir()
        .or_else(dirs::data_local_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mitm")
        .join("control.sock")
}

/// Binds the socket privately, replacing a stale file from a previous run.
pub fn bind(path: &Path) -> Result<UnixListener, String> {
    let dir = path.parent().ok_or("Control socket path has no directory")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;
    let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path).map_err(|e| format!("Failed to bind {}: {}", path.display(), e))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    Ok(listener)
}

pub async fn serve(core: MitmCore, listener: UnixListener, stop_rx: oneshot::Receiver<()>) {
    tokio::select! {
        _ = async {
            loop {
                let Ok((stream, _)) = listener.accept().await else { continue };
                tokio::spawn(handle_connection(core.clone(), stream));
            }
        } => {}
        _ = stop_rx => {}
    }
}

async fn handle_connection(core: MitmCore, stream: UnixStream) {
    let (read, mut write) = stream.into_split();
    let mut lines = BufReader::new(read).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        let mut reply = respond(&core, &line).await.to_string();
        reply.push('\n');
        if write.write_all(reply.as_bytes()).await.is_err() {
            return;
        }
    }
}

async fn respond(core: &MitmCore, line: &str) -> Value {
    let request: Value = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(e) => return json!({ "id": null, "error": format!("Invalid JSON: {e}") }),
    };
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let params = request.get("params").cloned().unwrap_or(Value::Null);
    match handlers::call(core, method, &params).await {
        Ok(result) => json!({ "id": id, "result": result }),
        Err(error) => json!({ "id": id, "error": error }),
    }
}
