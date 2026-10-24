use std::path::PathBuf;

use crate::{control, MitmCore};

impl MitmCore {
    /// Starts the control socket, replacing a previous one; returns the path it listens on.
    pub async fn start_control_server(&self) -> Result<PathBuf, String> {
        self.start_control_server_at(control::default_socket_path()).await
    }

    pub async fn start_control_server_at(&self, path: PathBuf) -> Result<PathBuf, String> {
        self.stop_control_server().await;
        let listener = control::bind(&path)?;
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
        *self.state.control_stop.lock().await = Some(stop_tx);
        tokio::spawn(control::serve(self.clone(), listener, stop_rx));
        Ok(path)
    }

    pub async fn stop_control_server(&self) {
        if let Some(stop) = self.state.control_stop.lock().await.take() {
            let _ = stop.send(());
        }
    }
}
