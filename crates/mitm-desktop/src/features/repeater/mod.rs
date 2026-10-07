mod view;
mod worker;

use mitm_core::repeater::execute::SentRequest;
use crate::command::RequestDraft;

pub use view::show;
pub use worker::RepeaterWorker;

#[derive(Debug, Clone)]
pub struct RepeaterTabState {
    pub id: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: String,
    pub body: String,
    pub is_loading: bool,
    pub last_result: Option<SentRequest>,
    pub error: Option<String>,
}

impl Default for RepeaterTabState {
    fn default() -> Self {
        Self::from_draft(RequestDraft {
            name: "New Request".to_string(),
            method: "GET".to_string(),
            url: "https://httpbin.org/get".to_string(),
            headers: "User-Agent: MITM-Native/0.2\nAccept: */*".to_string(),
            body: String::new(),
        })
    }
}

impl RepeaterTabState {
    fn from_draft(draft: RequestDraft) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: draft.name,
            method: draft.method,
            url: draft.url,
            headers: draft.headers,
            body: draft.body,
            is_loading: false,
            last_result: None,
            error: None,
        }
    }
}

pub struct RepeaterState {
    pub tabs: Vec<RepeaterTabState>,
    pub active: usize,
}

impl Default for RepeaterState {
    fn default() -> Self {
        Self {
            tabs: vec![RepeaterTabState::default()],
            active: 0,
        }
    }
}

impl RepeaterState {
    pub fn open(&mut self, draft: RequestDraft) {
        self.tabs.push(RepeaterTabState::from_draft(draft));
        self.active = self.tabs.len() - 1;
    }

    pub fn apply_result(&mut self, tab_id: &str, result: Result<SentRequest, String>) {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return;
        };
        tab.is_loading = false;
        match result {
            Ok(r) => {
                tab.last_result = Some(r);
                tab.error = None;
            }
            Err(e) => {
                tab.error = Some(e);
                tab.last_result = None;
            }
        }
    }
}
