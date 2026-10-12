use std::sync::Arc;
use crate::{db, state, webhook, MitmCore};

impl MitmCore {
    pub fn clear_webhook_deliveries(&self) -> Result<(), String> {
        db::webhook_db::clear_webhook_deliveries(&self.db_path)
    }

    pub async fn webhook_status(&self) -> state::WebhookListenerConfig {
        state::WebhookListenerConfig {
            port: *self.state.webhook_port.read().await,
            is_running: self.state.is_webhook_running(),
        }
    }

    pub async fn start_webhook_receiver(&self, port: u16) -> Result<(), String> {
        if port == 0 {
            return Err("Port must be between 1 and 65535".to_string());
        }
        let was_running = self.state.is_webhook_running();
        if was_running {
            self.stop_webhook_receiver().await?;
        }

        // A receiver we just stopped frees its port a moment later, so retry briefly in that case.
        let mut attempts = if was_running { 20 } else { 1 };
        let listener = loop {
            attempts -= 1;
            match webhook::bind_webhook_listener(port).await {
                Ok(l) => break l,
                Err(e) if attempts == 0 => return Err(e),
                Err(_) => tokio::time::sleep(std::time::Duration::from_millis(50)).await,
            }
        };
        self.state.set_webhook_running(true);

        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
        *self.state.webhook_stop_signal.lock().await = Some(stop_tx);
        *self.state.webhook_port.write().await = port;
        let _ = db::set_preference(&self.db_path, "webhook_port", &port.to_string());

        let bus = self.event_bus.clone();
        let state = Arc::clone(&self.state);
        tokio::spawn(async move {
            if let Err(e) = webhook::start_webhook_listener_server(Some(bus), state, listener, port, stop_rx).await {
                eprintln!("[Webhook Server] Error: {}", e);
            }
        });
        Ok(())
    }

    pub async fn stop_webhook_receiver(&self) -> Result<(), String> {
        if let Some(tx) = self.state.webhook_stop_signal.lock().await.take() {
            let _ = tx.send(());
        }
        self.state.set_webhook_running(false);
        Ok(())
    }

    /// Saves the port to use on the next start; a running receiver keeps its current port.
    pub async fn set_webhook_port(&self, port: u16) -> Result<(), String> {
        if port == 0 {
            return Err("Port must be between 1 and 65535".to_string());
        }
        if self.state.is_webhook_running() {
            return Err("Stop the receiver before changing its port".to_string());
        }
        *self.state.webhook_port.write().await = port;
        db::set_preference(&self.db_path, "webhook_port", &port.to_string())
    }

    pub fn webhook_endpoints(&self) -> Result<Vec<state::WebhookEndpoint>, String> {
        db::webhook_db::load_webhook_endpoints(&self.db_path)
    }

    pub fn create_webhook_endpoint(&self, name: String, path: String, secret: String) -> Result<state::WebhookEndpoint, String> {
        let endpoint = state::WebhookEndpoint {
            id: format!("wh-ep-{}", uuid::Uuid::new_v4()),
            name,
            path,
            secret_key: secret,
            created_at: chrono::Local::now().timestamp_millis(),
            hit_count: 0,
        };
        db::webhook_db::save_webhook_endpoint(&self.db_path, &endpoint)?;
        Ok(endpoint)
    }

    pub fn delete_webhook_endpoint(&self, id: &str) -> Result<(), String> {
        db::webhook_db::delete_webhook_endpoint(&self.db_path, id)
    }

    pub fn webhook_deliveries(&self, limit: Option<u32>) -> Result<Vec<state::WebhookDelivery>, String> {
        db::webhook_db::load_webhook_deliveries(&self.db_path, limit)
    }

    pub fn calculate_webhook_signature(&self, secret: &str, body: &str, provider: &str) -> state::WebhookSignatureResult {
        let (header_name, header_value) = webhook::calculate_signature(secret, body, provider);
        state::WebhookSignatureResult { header_name, header_value }
    }

    /// Re-verifies a stored delivery against its endpoint's current secret.
    /// Returns (status, expected, provided).
    pub fn verify_webhook(&self, delivery: &state::WebhookDelivery) -> Result<(String, Option<String>, Option<String>), String> {
        let secret = db::webhook_db::load_webhook_endpoints(&self.db_path)?
            .into_iter()
            .find(|e| e.id == delivery.endpoint_id)
            .map(|e| e.secret_key)
            .unwrap_or_default();
        Ok(webhook::verify_signature(&delivery.headers, delivery.payload.as_bytes(), &secret))
    }

    pub async fn replay_webhook(&self, delivery: state::WebhookDelivery, target_url: &str) -> Result<webhook::WebhookReplayResult, String> {
        webhook::replay_delivery(delivery.headers, delivery.payload, target_url).await
    }
}
