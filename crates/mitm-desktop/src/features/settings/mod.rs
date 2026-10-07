mod view;

use mitm_core::state::{ListenerConfig, ProxyMode};

pub use view::show;

pub struct ProxyState {
    pub mode: ProxyMode,
    pub running: bool,
    pub error: Option<String>,
    pub listeners: Vec<ListenerConfig>,
    pub active_summary: String,
}

impl Default for ProxyState {
    fn default() -> Self {
        Self {
            mode: ProxyMode::On,
            running: false,
            error: None,
            listeners: Vec::new(),
            active_summary: String::new(),
        }
    }
}

impl ProxyState {
    pub fn set_started(&mut self, result: Result<Vec<String>, String>) {
        match result {
            Ok(started) => {
                self.running = true;
                self.active_summary = started.join("  |  ");
                self.error = None;
            }
            Err(e) => {
                self.running = false;
                self.error = Some(e);
            }
        }
    }
}

pub struct ListenerForm {
    pub label: String,
    pub address: String,
}

impl Default for ListenerForm {
    fn default() -> Self {
        Self {
            label: String::new(),
            address: "127.0.0.1:8082".to_string(),
        }
    }
}
