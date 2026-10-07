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
}
