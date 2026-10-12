use std::sync::Arc;
use crate::{db, ws, MitmCore};

impl MitmCore {
    pub fn clear_ws_messages(&self) -> Result<(), String> {
        db::ws_db::clear_ws_messages(&self.db, None)
    }

    pub async fn connect_ws_client(
        &self,
        url: String,
        headers: Option<Vec<(String, String)>>,
    ) -> Result<crate::ws::types::WebSocketConn, String> {
        ws::connect_client_session(Some(self.event_bus.clone()), Arc::clone(&self.state), url, headers).await
    }

    pub async fn send_ws_frame(
        &self,
        connection_id: i64,
        msg_type: String,
        payload: String,
        to_server: bool,
    ) -> Result<(), String> {
        ws::send_frame(Some(self.event_bus.clone()), Arc::clone(&self.state), connection_id, &msg_type, payload, to_server).await
    }

    pub async fn disconnect_ws(&self, connection_id: i64) -> Result<(), String> {
        ws::disconnect_client_session(Arc::clone(&self.state), connection_id).await
    }

    pub fn delete_ws_connection(&self, connection_id: i64) -> Result<(), String> {
        db::ws_db::delete_ws_connection(&self.db, connection_id)
    }

    pub fn clear_ws_session(&self, connection_id: i64) -> Result<(), String> {
        db::ws_db::clear_ws_messages(&self.db, Some(connection_id))
    }

    pub async fn ws_mitm_enabled(&self) -> bool {
        self.state.proxy_config.read().await.ws_mitm_enabled
    }

    pub async fn set_ws_mitm_enabled(&self, enabled: bool) -> Result<(), String> {
        self.state.proxy_config.write().await.ws_mitm_enabled = enabled;
        db::set_preference(&self.db, "ws_mitm_enabled", if enabled { "true" } else { "false" })?;
        Ok(())
    }
}
