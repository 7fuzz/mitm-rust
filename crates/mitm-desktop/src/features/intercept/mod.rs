mod view;

use serde::{Deserialize, Serialize};
use mitm_core::db::InterceptRule;
use mitm_core::state::PendingFlowPayload;
use crate::format::{headers_to_text, parse_header_lines};

pub use view::show;

pub struct RuleDraft {
    pub phase: String,
    pub field: String,
    pub operator: String,
    pub value: String,
    pub action: String,
}

impl Default for RuleDraft {
    fn default() -> Self {
        Self {
            phase: "request".to_string(),
            field: "url".to_string(),
            operator: "contains".to_string(),
            value: String::new(),
            action: "intercept".to_string(),
        }
    }
}

#[derive(Default)]
pub struct FlowEdit {
    pub url: String,
    pub method: String,
    pub headers: String,
    pub body: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InterceptPrefs {
    pub work_fraction: f32,
    pub queue_fraction: f32,
}

impl Default for InterceptPrefs {
    fn default() -> Self {
        Self { work_fraction: 0.62, queue_fraction: 0.3 }
    }
}

pub struct InterceptState {
    pub prefs: InterceptPrefs,
    pub enabled: bool,
    pub mode: String,
    pub pending: Vec<PendingFlowPayload>,
    pub active_id: Option<String>,
    pub rules: Vec<InterceptRule>,
    pub edit: FlowEdit,
    pub new_rule: RuleDraft,
}

impl Default for InterceptState {
    fn default() -> Self {
        Self {
            prefs: InterceptPrefs::default(),
            enabled: false,
            mode: "request".to_string(),
            pending: Vec::new(),
            active_id: None,
            rules: Vec::new(),
            edit: FlowEdit {
                method: "GET".to_string(),
                ..Default::default()
            },
            new_rule: RuleDraft::default(),
        }
    }
}

impl InterceptState {
    pub fn add_pending(&mut self, payload: PendingFlowPayload) {
        let flow_id = payload.flow_id.clone();
        self.pending.push(payload);
        if self.active_id.is_none() {
            self.select(&flow_id);
        }
    }

    pub fn remove_pending(&mut self, flow_id: &str) {
        self.pending.retain(|p| p.flow_id != flow_id);
        if self.active_id.as_deref() != Some(flow_id) {
            return;
        }
        match self.pending.first().map(|p| p.flow_id.clone()) {
            Some(next) => self.select(&next),
            None => {
                self.active_id = None;
                self.edit.url.clear();
                self.edit.body.clear();
                self.edit.headers.clear();
            }
        }
    }

    pub fn clear_pending(&mut self) {
        self.pending.clear();
        self.active_id = None;
        self.edit = FlowEdit { method: self.edit.method.clone(), ..FlowEdit::default() };
    }

    pub fn select(&mut self, flow_id: &str) {
        if let Some(p) = self.pending.iter().find(|p| p.flow_id == flow_id) {
            self.active_id = Some(flow_id.to_string());
            self.edit = FlowEdit {
                url: p.url.clone(),
                method: p.method.clone(),
                headers: headers_to_text(&p.headers),
                body: p.body_text.clone(),
            };
        }
    }

    pub fn edited_headers(&self) -> Vec<(String, String)> {
        parse_header_lines(&self.edit.headers)
    }
}
