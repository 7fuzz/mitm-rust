mod view;
mod worker;

use crate::command::RequestDraft;

pub use view::show;
pub use worker::FuzzerWorker;

#[derive(Debug, Clone)]
pub struct FuzzResultRow {
    pub index: usize,
    pub payload: String,
    pub status_code: u16,
    pub size: u64,
    pub duration_ms: u64,
    pub error: Option<String>,
}

pub struct FuzzerState {
    pub method: String,
    pub url: String,
    pub headers: String,
    pub body: String,
    pub wordlist_input: String,
    pub concurrency: usize,
    pub is_running: bool,
    pub progress_completed: usize,
    pub progress_total: usize,
    pub results: Vec<FuzzResultRow>,
}

impl Default for FuzzerState {
    fn default() -> Self {
        Self {
            method: "GET".to_string(),
            url: "https://httpbin.org/status/{{status}}".to_string(),
            headers: "User-Agent: MITM-Native/0.2".to_string(),
            body: String::new(),
            wordlist_input: "200\n301\n400\n404\n500".to_string(),
            concurrency: 5,
            is_running: false,
            progress_completed: 0,
            progress_total: 0,
            results: Vec::new(),
        }
    }
}

impl FuzzerState {
    pub fn load_draft(&mut self, draft: RequestDraft) {
        self.method = draft.method;
        self.url = draft.url;
        self.headers = draft.headers;
        self.body = draft.body;
    }
}
