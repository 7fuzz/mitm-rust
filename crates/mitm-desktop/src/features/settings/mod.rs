mod view;

use std::collections::BTreeMap;
use mitm_core::state::{ListenerConfig, ProxyMode};

pub use view::show;

pub struct ProxyState {
    pub mode: ProxyMode,
    pub running: bool,
    pub error: Option<String>,
    pub listener_errors: BTreeMap<u32, String>,
    pub listeners: Vec<ListenerConfig>,
    pub active_summary: String,
}

impl Default for ProxyState {
    fn default() -> Self {
        Self {
            mode: ProxyMode::On,
            running: false,
            error: None,
            listener_errors: BTreeMap::new(),
            listeners: Vec::new(),
            active_summary: String::new(),
        }
    }
}

impl ProxyState {
    pub fn problems(&self) -> Vec<String> {
        let listener_problems = self.listener_errors.iter().map(|(id, error)| {
            let label = self.listeners.iter().find(|l| l.id == *id).map_or("", |l| l.label.as_str());
            format!("#{} {}: {}", id, label, error)
        });
        self.error.iter().cloned().chain(listener_problems).collect()
    }

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
