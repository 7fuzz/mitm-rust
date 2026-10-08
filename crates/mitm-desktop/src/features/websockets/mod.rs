mod view;

use std::path::PathBuf;
use mitm_core::state::{WebSocketConn, WebSocketMessage};

pub use view::show;

pub struct WebSocketsState {
    pub connections: Vec<WebSocketConn>,
    pub selected_id: Option<i64>,
    pub messages: Vec<WebSocketMessage>,
    pub direction_filter: String,
}

impl Default for WebSocketsState {
    fn default() -> Self {
        Self {
            connections: Vec::new(),
            selected_id: None,
            messages: Vec::new(),
            direction_filter: "ALL".to_string(),
        }
    }
}

impl WebSocketsState {
    pub fn load_connections(&mut self, db_path: &PathBuf) {
        if let Ok(conns) = mitm_core::db::ws_db::load_ws_connections(db_path) {
            self.connections = conns;
        }
    }

    pub fn select(&mut self, conn_id: i64, db_path: &PathBuf) {
        self.selected_id = Some(conn_id);
        if let Ok(msgs) = mitm_core::db::ws_db::load_ws_messages(db_path, conn_id, None) {
            self.messages = msgs;
        }
    }

    /// A `WsFrameReceived` payload is either a serialized connection (on open)
    /// or a single message. The two shapes share no required fields.
    pub fn apply_frame(&mut self, frame: serde_json::Value) {
        if let Ok(conn) = serde_json::from_value::<WebSocketConn>(frame.clone()) {
            self.upsert_connection(conn);
        } else if let Ok(msg) = serde_json::from_value::<WebSocketMessage>(frame) {
            self.apply_message(msg);
        }
    }

    fn upsert_connection(&mut self, conn: WebSocketConn) {
        match self.connections.iter_mut().find(|c| c.connection_id == conn.connection_id) {
            Some(existing) => *existing = conn,
            None => self.connections.push(conn),
        }
    }

    fn apply_message(&mut self, msg: WebSocketMessage) {
        if let Some(conn) = self.connections.iter_mut().find(|c| c.connection_id == msg.connection_id) {
            conn.message_count += 1;
        }
        if self.selected_id == Some(msg.connection_id) {
            self.messages.push(msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn(id: i64) -> serde_json::Value {
        serde_json::to_value(WebSocketConn {
            connection_id: id,
            url: format!("wss://example/{id}"),
            status: "connected".to_string(),
            handshake_time: 0,
            closed_at: None,
            protocol: None,
            client_addr: None,
            is_client_session: false,
            message_count: 0,
            listener_label: String::new(),
        })
        .unwrap()
    }

    fn message(conn_id: i64, payload: &str) -> serde_json::Value {
        serde_json::to_value(WebSocketMessage {
            id: 0,
            connection_id: conn_id,
            direction: "to_client".to_string(),
            msg_type: "text".to_string(),
            payload: payload.to_string(),
            timestamp: 0,
            length: payload.len(),
            is_injected: false,
        })
        .unwrap()
    }

    #[test]
    fn frames_add_connections_and_bump_counts() {
        let mut state = WebSocketsState::default();
        state.apply_frame(conn(1));
        state.apply_frame(conn(2));
        assert_eq!(state.connections.len(), 2);

        state.apply_frame(message(1, "hi"));
        assert_eq!(state.connections[0].message_count, 1);
        assert!(state.messages.is_empty(), "nothing selected, so no append");

        state.select_in_memory(1);
        state.apply_frame(message(1, "again"));
        assert_eq!(state.connections[0].message_count, 2);
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].payload, "again");

        state.apply_frame(message(2, "other session"));
        assert_eq!(state.messages.len(), 1, "frame for another session is not appended");
        assert_eq!(state.connections[1].message_count, 1);
    }

    impl WebSocketsState {
        fn select_in_memory(&mut self, conn_id: i64) {
            self.selected_id = Some(conn_id);
            self.messages.clear();
        }
    }
}
