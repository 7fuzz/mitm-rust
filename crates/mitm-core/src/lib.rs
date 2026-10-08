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
pub use ca::{CaFormat, CaInfo};
pub use db::{DataCategory, DataCategoryCounts};
pub use events::{CoreEvent, EventBus};
pub use state::{AppState, HistoryEntry, RewriteHistoryEntry};
pub use workspace::{Environment, EnvironmentVariable, Unresolved, VariableVariant, Workspace};

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

    pub fn ca_info(&self) -> Result<ca::CaInfo, String> {
        let ca_dir = ca::get_default_ca_dir();
        self.get_ca().info(&ca_dir)
    }

    pub fn ca_pem(&self) -> Result<String, String> {
        Ok(self.get_ca().cert_pem.clone())
    }

    pub fn export_ca(&self, path: &std::path::Path, format: ca::CaFormat) -> Result<(), String> {
        self.get_ca().export(path, format)
    }

    pub async fn regenerate_ca(&self) -> Result<(), String> {
        let ca_dir = ca::get_default_ca_dir();
        ca::delete_ca(ca_dir.clone());
        let _ = ca::get_ca(ca_dir);
        if self.state.proxy_active.load(std::sync::atomic::Ordering::SeqCst) {
            let _ = self.start_all_listeners().await?;
        }
        Ok(())
    }

    pub fn clear_all_traffic(&self) -> Result<(), String> {
        db::clear_history(&self.db_path)?;
        self.state.reset_history_id();
        self.event_bus.emit(CoreEvent::TrafficCleared);
        Ok(())
    }

    pub fn data_counts(&self) -> Result<db::DataCategoryCounts, String> {
        db::get_data_category_counts(&self.db_path)
    }

    pub fn clear_data_category(&self, category: db::DataCategory) -> Result<(), String> {
        match category {
            db::DataCategory::Traffic => self.clear_all_traffic(),
            db::DataCategory::WebhookDeliveries => self.clear_webhook_deliveries(),
            db::DataCategory::WsMessages => self.clear_ws_messages(),
            db::DataCategory::FuzzRuns => self.clear_fuzz_runs(),
            db::DataCategory::RewriteHistory => self.clear_rewrite_history(),
        }
    }

    pub fn clear_webhook_deliveries(&self) -> Result<(), String> {
        db::webhook_db::clear_webhook_deliveries(&self.db_path)
    }

    pub fn clear_ws_messages(&self) -> Result<(), String> {
        db::ws_db::clear_ws_messages(&self.db_path, None)
    }

    pub fn clear_fuzz_runs(&self) -> Result<(), String> {
        fuzzer::clear_fuzz_runs(&self.db_path)
    }

    pub fn clear_rewrite_history(&self) -> Result<(), String> {
        db::clear_rewrite_history_logs(&self.db_path)
    }

    pub async fn factory_reset(&self) -> Result<(), String> {
        self.stop_all_listeners().await;
        db::recovery::factory_reset(&self.db_path, Some(&self.event_bus)).map_err(|e| e.to_string())?;
        self.state.reset_history_id();
        self.event_bus.emit(CoreEvent::TrafficCleared);
        Ok(())
    }

    pub fn list_db_tables(&self) -> Result<Vec<db::browser::DbTable>, String> {
        db::browser::list_db_tables(&self.db_path)
    }

    pub fn query_db_table(
        &self,
        table: &str,
        offset: i64,
        limit: i64,
        order_by: Option<&str>,
        order_desc: Option<bool>,
        search: Option<&str>,
    ) -> Result<db::browser::DbRows, String> {
        db::browser::query_db_table(&self.db_path, table, offset, limit, order_by, order_desc, search)
    }

    pub fn get_db_row(&self, table: &str, rowid: i64) -> Result<Vec<serde_json::Value>, String> {
        db::browser::get_db_row(&self.db_path, table, rowid)
    }

    pub fn run_db_query(&self, sql: &str) -> Result<db::browser::DbQueryResult, String> {
        db::browser::run_db_query(&self.db_path, sql)
    }

    pub async fn start_all_listeners(&self) -> Result<Vec<String>, String> {
        let configs = self.state.listener_configs.read().await.clone();
        let ca = self.get_ca();
        self.stop_all_listeners().await;
        self.state.listener_errors.clear();

        let mut started = Vec::new();
        for config in configs.into_iter().filter(|c| c.enabled) {
            started.extend(self.start_listener(config, Arc::clone(&ca)).await);
        }
        self.state.proxy_active.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(started)
    }

    /// Binds one listener; a failure is recorded in `listener_errors`. Returns its summary when bound.
    async fn start_listener(&self, config: state::ListenerConfig, ca: Arc<ca::CA>) -> Option<String> {
        let addr: std::net::SocketAddr = match config.address.parse() {
            Ok(a) => a,
            Err(e) => {
                self.state.listener_errors.insert(config.id, format!("Invalid address: {}", e));
                return None;
            }
        };
        let tcp_listener = match proxy::bind_proxy_listener(addr).await {
            Ok(l) => l,
            Err(e) => {
                self.state.listener_errors.insert(config.id, e);
                return None;
            }
        };

        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
        self.state.listeners.write().await.push(state::ListenerInstance {
            id: config.id,
            stop_tx: Some(stop_tx),
        });

        let event_bus = Some(self.event_bus.clone());
        let state = Arc::clone(&self.state);
        let label = config.label.clone();
        tokio::spawn(async move {
            let _ = proxy::start_proxy_server(event_bus, state, ca, tcp_listener, label, stop_rx).await;
        });
        Some(format!("{} ({})", config.label, config.address))
    }

    async fn stop_listener(&self, id: u32) {
        let mut listeners = self.state.listeners.write().await;
        listeners.retain_mut(|instance| {
            if instance.id != id {
                return true;
            }
            if let Some(tx) = instance.stop_tx.take() {
                let _ = tx.send(());
            }
            false
        });
    }

    /// Turns one listener on or off without restarting the others.
    pub async fn set_listener_enabled(&self, id: u32, enabled: bool) -> Result<(), String> {
        let config = {
            let mut configs = self.state.listener_configs.write().await;
            let config = configs.iter_mut().find(|c| c.id == id).ok_or_else(|| format!("No listener #{}", id))?;
            config.enabled = enabled;
            let config = config.clone();
            let json = serde_json::to_string(&*configs).map_err(|e| e.to_string())?;
            db::set_preference(&self.db_path, "proxy_listeners", &json)?;
            config
        };
        self.stop_listener(id).await;
        self.state.listener_errors.remove(&id);
        if enabled && self.state.proxy_active.load(std::sync::atomic::Ordering::SeqCst) {
            self.start_listener(config, self.get_ca()).await;
        }
        Ok(())
    }

    pub async fn listener_status(&self) -> Vec<state::ListenerStatus> {
        let configs = self.state.listener_configs.read().await.clone();
        let running = self.state.listeners.read().await;
        configs
            .into_iter()
            .map(|config| state::ListenerStatus {
                running: running.iter().any(|l| l.id == config.id),
                error: self.state.listener_errors.get(&config.id).map(|e| e.clone()),
                config,
            })
            .collect()
    }

    pub fn list_host_ips(&self) -> Result<Vec<proxy::host_ips::HostIp>, String> {
        proxy::host_ips::list()
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

    pub async fn proxy_mode(&self) -> state::ProxyMode {
        let mode = self.state.proxy_config.read().await.proxy_mode.clone();
        state::ProxyMode::parse(&mode).unwrap_or(state::ProxyMode::On)
    }

    /// Saves the mode and starts or stops the listeners to match. Returns the listeners it started, if any.
    pub async fn set_proxy_mode(&self, mode: state::ProxyMode) -> Result<Option<Vec<String>>, String> {
        {
            let mut cfg = self.state.proxy_config.write().await;
            cfg.proxy_mode = mode.as_str().to_string();
            cfg.proxy_enabled = mode.is_enabled();
        }
        db::set_preference(&self.db_path, "proxy_mode", mode.as_str())?;
        db::set_preference(&self.db_path, "proxy_enabled", if mode.is_enabled() { "true" } else { "false" })?;

        let active = self.state.proxy_active.load(std::sync::atomic::Ordering::SeqCst);
        if mode.is_enabled() && !active {
            return self.start_all_listeners().await.map(Some);
        }
        if !mode.is_enabled() && active {
            self.stop_all_listeners().await;
        }
        Ok(None)
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

    /// The interceptor settings as they were loaded at startup or last set.
    pub async fn intercept_config(&self) -> (bool, String, state::SourceScope) {
        let cfg = self.state.proxy_config.read().await;
        (cfg.intercept_enabled, cfg.intercept_mode.clone(), cfg.intercept_source_scope.clone())
    }

    pub async fn set_intercept_source_scope(&self, scope: state::SourceScope) -> Result<(), String> {
        let json = serde_json::to_string(&scope).map_err(|e| e.to_string())?;
        db::set_preference(&self.db_path, "intercept_source_scope", &json)?;
        self.state.proxy_config.write().await.intercept_source_scope = scope;
        Ok(())
    }

    /// Forwards every paused flow unmodified and returns how many were released.
    pub fn forward_all_intercepted(&self) -> usize {
        self.pending_flow_ids()
            .iter()
            .filter(|id| self.forward_intercepted_flow(id, None, None, None, None).is_ok())
            .count()
    }

    pub fn drop_all_intercepted(&self) -> usize {
        self.pending_flow_ids().iter().filter(|id| self.drop_intercepted_flow(id).is_ok()).count()
    }

    fn pending_flow_ids(&self) -> Vec<String> {
        self.state.pending_flows.iter().map(|entry| entry.key().clone()).collect()
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

    pub fn get_workspaces(&self) -> Result<Vec<workspace::Workspace>, String> {
        workspace::get_workspaces_db(&self.db_path)
    }

    pub fn get_workspace(&self, id: &str) -> Result<Option<workspace::Workspace>, String> {
        workspace::get_workspace_db(&self.db_path, id)
    }

    pub fn get_active_workspace(&self) -> Result<Option<workspace::Workspace>, String> {
        workspace::get_active_workspace_db(&self.db_path)
    }

    pub fn set_active_workspace(&self, id: &str) -> Result<(), String> {
        workspace::set_active_workspace_db(&self.db_path, id)
    }

    pub fn create_workspace(&self, name: &str, description: Option<&str>) -> Result<workspace::Workspace, String> {
        workspace::create_workspace_db(&self.db_path, name.to_string(), description.map(|s| s.to_string()))
    }

    pub fn rename_workspace(&self, id: &str, new_name: &str) -> Result<(), String> {
        workspace::rename_workspace_db(&self.db_path, id, new_name)
    }

    pub fn update_workspace_description(&self, id: &str, description: Option<&str>) -> Result<(), String> {
        workspace::update_workspace_description_db(&self.db_path, id, description)
    }

    pub fn delete_workspace(&self, id: &str) -> Result<(), String> {
        workspace::delete_workspace_db(&self.db_path, id)
    }

    pub fn get_workspace_environments(&self, workspace_id: &str) -> Result<Vec<workspace::Environment>, String> {
        workspace::get_workspace_environments_db(&self.db_path, workspace_id)
    }

    pub fn get_workspace_environment(&self, id: &str) -> Result<Option<workspace::Environment>, String> {
        workspace::get_workspace_environment_db(&self.db_path, id)
    }

    pub fn save_workspace_environment(&self, env: workspace::Environment) -> Result<(), String> {
        workspace::save_workspace_environment_db(&self.db_path, env)
    }

    pub fn delete_workspace_environment(&self, id: &str) -> Result<(), String> {
        workspace::delete_workspace_environment_db(&self.db_path, id)
    }

    pub fn duplicate_workspace_environment(&self, id: &str, new_name: Option<&str>) -> Result<workspace::Environment, String> {
        workspace::duplicate_workspace_environment_db(&self.db_path, id, new_name)
    }

    pub fn set_active_environment(&self, workspace_id: &str, env_id: Option<&str>) -> Result<(), String> {
        workspace::set_active_environment_db(&self.db_path, workspace_id, env_id)
    }

    pub fn get_active_environment(&self) -> Result<Option<workspace::Environment>, String> {
        if let Some(ws) = self.get_active_workspace()? {
            let envs = self.get_workspace_environments(&ws.id)?;
            if let Some(active_id) = &ws.active_environment_id {
                if let Some(env) = envs.iter().find(|e| &e.id == active_id) {
                    return Ok(Some(env.clone()));
                }
            }
            if let Some(env) = envs.into_iter().find(|e| e.is_active) {
                return Ok(Some(env));
            }
        }
        Ok(None)
    }

    pub fn resolve_vars(&self, text: &str) -> (String, Vec<workspace::Unresolved>) {
        let active_env = self.get_active_environment().ok().flatten();
        workspace::resolve_variables_with_env(active_env.as_ref(), text)
    }

    pub fn variable_value(&self, name: &str) -> Option<String> {
        let active_env = self.get_active_environment().ok().flatten();
        workspace::variable_value_in_env(active_env.as_ref(), name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::oneshot;

    fn test_core() -> (MitmCore, PathBuf) {
        let dir = std::env::temp_dir().join(format!("mitm-core-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("test.db");
        db::init_database(&db_path, None).unwrap();
        let (history_tx, _) = mpsc::channel(1);
        let (rewrite_tx, _) = mpsc::channel(1);
        let state = Arc::new(AppState::new(db_path.clone(), history_tx, rewrite_tx));
        (MitmCore { state, event_bus: EventBus::default(), db_path }, dir)
    }

    fn pause(core: &MitmCore, id: &str) -> oneshot::Receiver<state::InterceptAction> {
        let (tx, rx) = oneshot::channel();
        core.state.pending_flows.insert(
            id.to_string(),
            state::PendingFlow {
                flow_id: id.to_string(),
                phase: state::InterceptPhase::Request,
                method: "GET".to_string(),
                url: "http://a".to_string(),
                headers: vec![],
                body: vec![],
                tx,
            },
        );
        rx
    }

    #[test]
    fn interceptor_settings_survive_a_restart() {
        let (core, dir) = test_core();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            core.toggle_interceptor(true, "both".to_string()).await.unwrap();
            core.set_intercept_source_scope(state::SourceScope { all: false, ids: vec![2] }).await.unwrap();
        });
        let (history_tx, _) = mpsc::channel(1);
        let (rewrite_tx, _) = mpsc::channel(1);
        let reopened = MitmCore {
            state: Arc::new(AppState::new(core.db_path.clone(), history_tx, rewrite_tx)),
            event_bus: EventBus::default(),
            db_path: core.db_path.clone(),
        };
        let (enabled, mode, scope) = rt.block_on(reopened.intercept_config());
        assert!(enabled);
        assert_eq!(mode, "both");
        assert!(!scope.all && scope.ids == vec![2]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn forward_all_and_drop_all_release_every_paused_flow() {
        let (core, dir) = test_core();
        let mut forwarded = [pause(&core, "a"), pause(&core, "b")];
        assert_eq!(core.forward_all_intercepted(), 2);
        assert!(forwarded.iter_mut().all(|rx| matches!(rx.try_recv(), Ok(state::InterceptAction::Forward { modified_url: None, .. }))));

        let mut dropped = pause(&core, "c");
        assert_eq!(core.drop_all_intercepted(), 1);
        assert!(matches!(dropped.try_recv(), Ok(state::InterceptAction::Drop)));
        assert!(core.state.pending_flows.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
