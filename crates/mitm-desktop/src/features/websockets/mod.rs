mod connect;
mod view;

use std::path::PathBuf;
use mitm_core::state::{WebSocketConn, WebSocketMessage};
use serde::{Deserialize, Serialize};

pub use view::show;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebSocketsPrefs {
    pub list_split: f32,
    pub detail_split: f32,
}

impl Default for WebSocketsPrefs {
    fn default() -> Self {
        Self { list_split: 0.4, detail_split: 0.6 }
    }
}

pub struct WebSocketsState {
    pub prefs: WebSocketsPrefs,
    pub connections: Vec<WebSocketConn>,
    pub selected_id: Option<i64>,
    pub messages: Vec<WebSocketMessage>,
    pub selected_msg: Option<i64>,
    pub detail_pane: crate::widgets::message_pane::PaneState,
    pub direction_filter: String,
    pub source_filter: String,
    pub frame_search: String,
    pub connect: connect::WsConnect,
}

impl Default for WebSocketsState {
    fn default() -> Self {
        Self {
            prefs: WebSocketsPrefs::default(),
            connections: Vec::new(),
            selected_id: None,
            messages: Vec::new(),
            selected_msg: None,
            detail_pane: Default::default(),
            direction_filter: "ALL".to_string(),
            source_filter: "ALL".to_string(),
            frame_search: String::new(),
            connect: Default::default(),
        }
    }
}

/// The source chip label for a connection: a client session, an unlabeled
/// proxy, or the listener it came through.
pub fn source_label(conn: &WebSocketConn) -> String {
    if conn.is_client_session {
        "Client".to_string()
    } else if conn.listener_label.is_empty() {
        "Proxy".to_string()
    } else {
        conn.listener_label.clone()
    }
}

/// Distinct source labels in first-seen order.
pub fn sources(conns: &[WebSocketConn]) -> Vec<String> {
    let mut seen = Vec::new();
    for conn in conns {
        let label = source_label(conn);
        if !seen.contains(&label) {
            seen.push(label);
        }
    }
    seen
}

impl WebSocketsState {
    pub fn load_connections(&mut self, db_path: &PathBuf) {
        if let Ok(conns) = mitm_core::db::ws_db::load_ws_connections(db_path) {
            self.connections = conns;
        }
    }

    pub fn select(&mut self, conn_id: i64, db_path: &PathBuf) {
        self.selected_id = Some(conn_id);
        self.selected_msg = None;
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

    /// Frames can arrive before the connect result does, so the session's frames
    /// and count come from the DB rather than starting empty.
    pub fn connected(&mut self, conn: WebSocketConn, db_path: &PathBuf) {
        let id = conn.connection_id;
        self.upsert_connection(conn);
        self.select(id, db_path);
        let count = self.messages.len() as u64;
        if let Some(conn) = self.connections.iter_mut().find(|c| c.connection_id == id) {
            conn.message_count = conn.message_count.max(count);
        }
    }

    fn upsert_connection(&mut self, conn: WebSocketConn) {
        match self.connections.iter_mut().find(|c| c.connection_id == conn.connection_id) {
            Some(existing) => *existing = conn,
            None => self.connections.push(conn),
        }
    }

    fn apply_message(&mut self, msg: WebSocketMessage) {
        let selected = self.selected_id == Some(msg.connection_id);
        // A frame stored just before a DB reload also arrives as an event afterwards.
        if selected && self.messages.iter().rev().take(32).any(|m| m.id == msg.id) {
            return;
        }
        if let Some(conn) = self.connections.iter_mut().find(|c| c.connection_id == msg.connection_id) {
            conn.message_count += 1;
        }
        if selected {
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

    fn message(id: i64, conn_id: i64, payload: &str) -> serde_json::Value {
        serde_json::to_value(WebSocketMessage {
            id,
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

        state.apply_frame(message(10, 1, "hi"));
        assert_eq!(state.connections[0].message_count, 1);
        assert!(state.messages.is_empty(), "nothing selected, so no append");

        state.select_in_memory(1);
        state.apply_frame(message(11, 1, "again"));
        assert_eq!(state.connections[0].message_count, 2);
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].payload, "again");

        state.apply_frame(message(11, 1, "again"));
        assert_eq!(state.messages.len(), 1, "a frame already loaded is not appended twice");
        assert_eq!(state.connections[0].message_count, 2);

        state.apply_frame(message(12, 2, "other session"));
        assert_eq!(state.messages.len(), 1, "frame for another session is not appended");
        assert_eq!(state.connections[1].message_count, 1);
    }

    #[test]
    fn source_labels_are_distinct_in_order() {
        let mk = |client: bool, label: &str| WebSocketConn {
            connection_id: 0,
            url: String::new(),
            status: String::new(),
            handshake_time: 0,
            closed_at: None,
            protocol: None,
            client_addr: None,
            is_client_session: client,
            message_count: 0,
            listener_label: label.to_string(),
        };
        let conns = vec![mk(false, "HTTP 8080"), mk(true, ""), mk(false, "HTTP 8080"), mk(false, "")];
        assert_eq!(sources(&conns), vec!["HTTP 8080", "Client", "Proxy"]);
    }

    impl WebSocketsState {
        fn select_in_memory(&mut self, conn_id: i64) {
            self.selected_id = Some(conn_id);
            self.messages.clear();
        }
    }
}
