pub mod ca;
pub mod collections;
pub mod db;
pub mod encoding;
pub mod events;
pub mod fuzzer;
pub mod instance;
pub mod proxy;
pub mod repeater;
pub mod request_steps;
pub mod state;
pub mod webhook;
pub mod workspace;
pub mod ws;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
pub use events::{CoreEvent, EventBus};
pub use state::{AppState, HistoryEntry, RewriteHistoryEntry};

pub struct MitmCore {
    pub state: Arc<AppState>,
    pub event_bus: EventBus,
    pub db_path: PathBuf,
}

impl MitmCore {
    pub fn init() -> Result<Self, String> {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let db_path = db::get_default_db_path();
        let event_bus = EventBus::default();

        db::init_database(&db_path, Some(&event_bus))?;

        let (history_tx, history_rx) = mpsc::channel::<HistoryEntry>(2000);
        let (rewrite_tx, rewrite_rx) = mpsc::channel::<RewriteHistoryEntry>(2000);

        db::actor::start_history_actor(db_path.clone(), history_rx);
        db::actor::start_rewrite_history_actor(db_path.clone(), rewrite_rx);

        let state = Arc::new(AppState::new(db_path.clone(), history_tx, rewrite_tx));

        Ok(Self {
            state,
            event_bus,
            db_path,
        })
    }

    pub fn get_ca(&self) -> Arc<ca::CA> {
        Arc::new(ca::get_ca(ca::get_default_ca_dir()))
    }

    pub async fn start_proxy(
        &self,
        port: u16,
        label: String,
        stop_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<(), String> {
        let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port)
            .parse()
            .map_err(|e| format!("Invalid address: {}", e))?;
        let listener = proxy::bind_proxy_listener(addr).await?;
        let ca = self.get_ca();
        proxy::start_proxy_server(
            Some(self.event_bus.clone()),
            Arc::clone(&self.state),
            ca,
            listener,
            label,
            stop_rx,
        ).await
    }

    pub async fn toggle_interceptor(&self, enabled: bool, mode: String) -> Result<(), String> {
        {
            let mut cfg = self.state.proxy_config.write().await;
            cfg.intercept_enabled = enabled;
            cfg.intercept_mode = mode.clone();
        }
        let _ = db::set_preference(&self.db_path, "intercept_enabled", if enabled { "true" } else { "false" });
        let _ = db::set_preference(&self.db_path, "intercept_mode", &mode);
        Ok(())
    }

    pub fn forward_intercepted_flow(
        &self,
        flow_id: &str,
        modified_url: Option<String>,
        modified_method: Option<String>,
        modified_headers: Option<Vec<(String, String)>>,
        modified_body: Option<Vec<u8>>,
    ) -> Result<(), String> {
        if let Some((_, pending)) = self.state.pending_flows.remove(flow_id) {
            let action = state::InterceptAction::Forward {
                modified_url,
                modified_method,
                modified_headers,
                modified_body,
            };
            let _ = pending.tx.send(action);
            self.event_bus.emit(events::CoreEvent::InterceptResolved { flow_id: flow_id.to_string() });
            Ok(())
        } else {
            Err(format!("Flow with ID {} not found", flow_id))
        }
    }

    pub fn drop_intercepted_flow(&self, flow_id: &str) -> Result<(), String> {
        if let Some((_, pending)) = self.state.pending_flows.remove(flow_id) {
            let _ = pending.tx.send(state::InterceptAction::Drop);
            self.event_bus.emit(events::CoreEvent::InterceptResolved { flow_id: flow_id.to_string() });
            Ok(())
        } else {
            Err(format!("Flow with ID {} not found", flow_id))
        }
    }

    pub async fn get_intercept_rules(&self) -> Vec<db::InterceptRule> {
        self.state.rules.read().await.clone()
    }

    pub async fn update_intercept_rules(&self, rules: Vec<db::InterceptRule>) -> Result<(), String> {
        db::save_intercept_rules(&self.db_path, &rules)?;
        let mut r = self.state.rules.write().await;
        *r = rules;
        Ok(())
    }
}
