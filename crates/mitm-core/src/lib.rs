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
    pub fn init() -> Result<Self, db::InitError> {
        Self::init_with_events(EventBus::default())
    }

    /// Like `init`, but on a bus the caller already subscribed to, so migration progress is observable.
    pub fn init_with_events(event_bus: EventBus) -> Result<Self, db::InitError> {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let db_path = db::get_default_db_path();

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

    pub fn clear_all_traffic(&self) -> Result<(), String> {
        db::clear_history(&self.db_path)?;
        self.state.reset_history_id();
        self.event_bus.emit(CoreEvent::TrafficCleared);
        Ok(())
    }

    pub async fn start_all_listeners(&self) -> Result<Vec<String>, String> {
        let configs = self.state.listener_configs.read().await.clone();
        let ca = self.get_ca();
        self.stop_all_listeners().await;

        let mut started = Vec::new();

        for config in configs.into_iter().filter(|c| c.enabled) {
            let addr: std::net::SocketAddr = match config.address.parse() {
                Ok(a) => a,
                Err(e) => {
                    self.state.listener_errors.insert(config.id, format!("Invalid address: {}", e));
                    continue;
                }
            };
            let tcp_listener = match proxy::bind_proxy_listener(addr).await {
                Ok(l) => l,
                Err(e) => {
                    self.state.listener_errors.insert(config.id, e);
                    continue;
                }
            };

            let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
            self.state.listeners.write().await.push(state::ListenerInstance {
                id: config.id,
                stop_tx: Some(stop_tx),
            });

            let event_bus = Some(self.event_bus.clone());
            let state = Arc::clone(&self.state);
            let ca_clone = Arc::clone(&ca);
            let label = config.label.clone();
            started.push(format!("{} ({})", config.label, config.address));

            tokio::spawn(async move {
                let _ = proxy::start_proxy_server(event_bus, state, ca_clone, tcp_listener, label, stop_rx).await;
            });
        }
        self.state.proxy_active.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(started)
    }

    pub async fn stop_all_listeners(&self) {
        let mut listeners = self.state.listeners.write().await;
        for instance in listeners.drain(..) {
            if let Some(tx) = instance.stop_tx {
                let _ = tx.send(());
            }
        }
        self.state.proxy_active.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub async fn get_listener_configs(&self) -> Vec<state::ListenerConfig> {
        self.state.listener_configs.read().await.clone()
    }

    pub async fn set_listener_configs(&self, configs: Vec<state::ListenerConfig>) -> Result<(), String> {
        let json = serde_json::to_string(&configs).map_err(|e| e.to_string())?;
        db::set_preference(&self.db_path, "proxy_listeners", &json)?;
        {
            let mut w = self.state.listener_configs.write().await;
            *w = configs;
        }
        if self.state.proxy_active.load(std::sync::atomic::Ordering::SeqCst) {
            let _ = self.start_all_listeners().await;
        }
        Ok(())
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

    pub async fn get_history_settings(&self) -> state::HistorySettings {
        self.state.history_settings.read().await.clone()
    }

    pub async fn set_history_settings(&self, settings: state::HistorySettings) -> Result<(), String> {
        db::set_preference(&self.db_path, "history_limiter_enabled", if settings.limiter_enabled { "true" } else { "false" })?;
        db::set_preference(&self.db_path, "history_limiter_max_rows", &settings.max_rows.to_string())?;
        if settings.limiter_enabled {
            db::prune_history_logs(&self.db_path, settings.max_rows)?;
        }
        *self.state.history_settings.write().await = settings;
        Ok(())
    }
}
