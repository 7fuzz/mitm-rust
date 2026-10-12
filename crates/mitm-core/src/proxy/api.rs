use std::sync::Arc;
use crate::{ca, db, events, proxy, MitmCore, RewriteHistoryEntry};

impl MitmCore {
    pub fn clear_rewrite_history(&self) -> Result<(), String> {
        db::clear_rewrite_history_logs(&self.db)
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
    async fn start_listener(&self, config: crate::proxy::listener::ListenerConfig, ca: Arc<ca::CA>) -> Option<String> {
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
        self.state.listeners.write().await.push(crate::proxy::listener::ListenerInstance {
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
            db::set_preference(&self.db, "proxy_listeners", &json)?;
            config
        };
        self.stop_listener(id).await;
        self.state.listener_errors.remove(&id);
        if enabled && self.state.proxy_active.load(std::sync::atomic::Ordering::SeqCst) {
            self.start_listener(config, self.get_ca()).await;
        }
        Ok(())
    }

    pub async fn listener_status(&self) -> Vec<crate::proxy::listener::ListenerStatus> {
        let configs = self.state.listener_configs.read().await.clone();
        let running = self.state.listeners.read().await;
        configs
            .into_iter()
            .map(|config| crate::proxy::listener::ListenerStatus {
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

    pub async fn proxy_mode(&self) -> crate::proxy::listener::ProxyMode {
        let mode = self.state.proxy_config.read().await.proxy_mode.clone();
        crate::proxy::listener::ProxyMode::parse(&mode).unwrap_or(crate::proxy::listener::ProxyMode::On)
    }

    /// Saves the mode and starts or stops the listeners to match. Returns the listeners it started, if any.
    pub async fn set_proxy_mode(&self, mode: crate::proxy::listener::ProxyMode) -> Result<Option<Vec<String>>, String> {
        {
            let mut cfg = self.state.proxy_config.write().await;
            cfg.proxy_mode = mode.as_str().to_string();
            cfg.proxy_enabled = mode.is_enabled();
        }
        db::set_preference(&self.db, "proxy_mode", mode.as_str())?;
        db::set_preference(&self.db, "proxy_enabled", if mode.is_enabled() { "true" } else { "false" })?;

        let active = self.state.proxy_active.load(std::sync::atomic::Ordering::SeqCst);
        if mode.is_enabled() && !active {
            return self.start_all_listeners().await.map(Some);
        }
        if !mode.is_enabled() && active {
            self.stop_all_listeners().await;
        }
        Ok(None)
    }

    pub async fn get_listener_configs(&self) -> Vec<crate::proxy::listener::ListenerConfig> {
        self.state.listener_configs.read().await.clone()
    }

    pub async fn set_listener_configs(&self, configs: Vec<crate::proxy::listener::ListenerConfig>) -> Result<(), String> {
        let json = serde_json::to_string(&configs).map_err(|e| e.to_string())?;
        db::set_preference(&self.db, "proxy_listeners", &json)?;
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
        let _ = db::set_preference(&self.db, "intercept_enabled", if enabled { "true" } else { "false" });
        let _ = db::set_preference(&self.db, "intercept_mode", &mode);
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
            let action = crate::proxy::intercept::InterceptAction::Forward {
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
            let _ = pending.tx.send(crate::proxy::intercept::InterceptAction::Drop);
            self.event_bus.emit(events::CoreEvent::InterceptResolved { flow_id: flow_id.to_string() });
            Ok(())
        } else {
            Err(format!("Flow with ID {} not found", flow_id))
        }
    }

    pub async fn rewrite_rules(&self) -> Vec<crate::proxy::rewrite::RewriteRule> {
        self.state.rewrite_rules.read().await.clone()
    }

    /// Saves rules in the given order, which also covers reordering.
    pub async fn save_rewrite_rules(&self, mut rules: Vec<crate::proxy::rewrite::RewriteRule>) -> Result<(), String> {
        for (index, rule) in rules.iter_mut().enumerate() {
            rule.order_index = index as i32;
        }
        db::save_rewrite_rules(&self.db, &rules)?;
        *self.state.rewrite_rules.write().await = rules;
        Ok(())
    }

    pub fn rewrite_enabled(&self) -> bool {
        self.state.is_rewrite_enabled()
    }

    pub fn set_rewrite_enabled(&self, enabled: bool) -> Result<(), String> {
        db::set_preference(&self.db, "rewrite_enabled", if enabled { "true" } else { "false" })?;
        self.state.set_rewrite_enabled(enabled);
        Ok(())
    }

    pub async fn rewrite_source_scope(&self) -> crate::proxy::rules::SourceScope {
        self.state.rewrite_source_scope.read().await.clone()
    }

    pub async fn set_rewrite_source_scope(&self, scope: crate::proxy::rules::SourceScope) -> Result<(), String> {
        let json = serde_json::to_string(&scope).map_err(|e| e.to_string())?;
        db::set_preference(&self.db, "rewrite_source_scope", &json)?;
        *self.state.rewrite_source_scope.write().await = scope;
        Ok(())
    }

    pub fn rewrite_history(&self, limit: u32) -> Result<Vec<RewriteHistoryEntry>, String> {
        db::get_rewrite_history_logs(&self.db, limit)
    }

    /// The interceptor settings as they were loaded at startup or last set.
    pub async fn intercept_config(&self) -> (bool, String, crate::proxy::rules::SourceScope) {
        let cfg = self.state.proxy_config.read().await;
        (cfg.intercept_enabled, cfg.intercept_mode.clone(), cfg.intercept_source_scope.clone())
    }

    pub async fn set_intercept_source_scope(&self, scope: crate::proxy::rules::SourceScope) -> Result<(), String> {
        let json = serde_json::to_string(&scope).map_err(|e| e.to_string())?;
        db::set_preference(&self.db, "intercept_source_scope", &json)?;
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
        db::save_intercept_rules(&self.db, &rules)?;
        let mut r = self.state.rules.write().await;
        *r = rules;
        Ok(())
    }

    pub async fn get_history_settings(&self) -> crate::proxy::history::HistorySettings {
        self.state.history_settings.read().await.clone()
    }

    pub async fn set_history_settings(&self, settings: crate::proxy::history::HistorySettings) -> Result<(), String> {
        db::set_preference(&self.db, "history_limiter_enabled", if settings.limiter_enabled { "true" } else { "false" })?;
        db::set_preference(&self.db, "history_limiter_max_rows", &settings.max_rows.to_string())?;
        if settings.limiter_enabled {
            db::prune_history_logs(&self.db, settings.max_rows)?;
        }
        *self.state.history_settings.write().await = settings;
        Ok(())
    }
}
