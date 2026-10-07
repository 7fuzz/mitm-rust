mod view;

use mitm_core::state::ListenerConfig;

pub use view::show;

#[derive(Default)]
pub struct ProxyState {
    pub running: bool,
    pub error: Option<String>,
    pub listeners: Vec<ListenerConfig>,
    pub active_summary: String,
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
