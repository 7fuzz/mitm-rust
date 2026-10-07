mod view;

use std::collections::{BTreeMap, BTreeSet};
use serde::{Deserialize, Serialize};
use mitm_core::state::{ListenerConfig, ProxyMode};

pub use view::show;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SettingsSection {
    Proxy,
    Certificate,
    Appearance,
    Data,
    Database,
}

impl SettingsSection {
    pub const ALL: [SettingsSection; 5] = [
        SettingsSection::Proxy,
        SettingsSection::Certificate,
        SettingsSection::Appearance,
        SettingsSection::Data,
        SettingsSection::Database,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SettingsSection::Proxy => "Proxy",
            SettingsSection::Certificate => "Certificate",
            SettingsSection::Appearance => "Appearance",
            SettingsSection::Data => "Data",
            SettingsSection::Database => "Database",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SettingsPrefs {
    pub section: SettingsSection,
    pub list_fraction: f32,
}

impl Default for SettingsPrefs {
    fn default() -> Self {
        Self { section: SettingsSection::Proxy, list_fraction: 0.18 }
    }
}

pub struct ProxyState {
    pub mode: ProxyMode,
    pub running: bool,
    pub error: Option<String>,
    pub listener_errors: BTreeMap<u32, String>,
    pub running_listeners: BTreeSet<u32>,
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
            running_listeners: BTreeSet::new(),
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
