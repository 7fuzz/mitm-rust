mod builder;
mod curl_import;
mod sidebar;
mod view;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use mitm_core::curl::ParsedCurl;
use mitm_core::repeater::{
    self as core_repeater, HeaderItem, RepeaterExecutionResult, RepeaterHistoryItem, RepeaterTab,
};
use mitm_core::state::{HistoryDetail, HistoryEntry};
use crate::features::request_editor::captured::{header_items, CapturedRequest};
use crate::features::request_editor::run_list::Run;
use crate::features::request_editor::url_params::{new_id, parse_query};
use crate::features::request_editor::run_list::RunDetails;
use crate::widgets::message_pane::PaneState;

pub use curl_import::{show as show_curl_import, CurlImport};
pub use view::show;

const SAVE_DELAY_SECS: f64 = 0.6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTab {
    Params,
    Headers,
    Body,
    Extract,
    Steps,
}

pub struct ShownResponse {
    pub history_id: i64,
    pub status_code: u16,
    pub status_text: String,
    pub headers: Vec<HeaderItem>,
    pub body: String,
    pub duration_ms: u64,
    pub size: u64,
}

impl ShownResponse {
    fn from_result(result: RepeaterExecutionResult) -> Self {
        Self {
            history_id: result.history_id,
            status_code: result.status_code,
            status_text: result.status_text,
            headers: result.response_headers,
            body: result.response_body,
            duration_ms: result.duration_ms,
            size: result.response_size,
        }
    }

    fn from_run(run: &RepeaterHistoryItem) -> Self {
        let body = run.response_body.clone().unwrap_or_default();
        Self {
            history_id: run.id,
            status_code: run.status_code,
            status_text: String::new(),
            headers: run.response_headers.clone(),
            size: body.len() as u64,
            body,
            duration_ms: run.duration_ms,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RepeaterPrefs {
    pub split: f32,
    pub history_open: bool,
}

impl Default for RepeaterPrefs {
    fn default() -> Self {
        Self { split: 0.5, history_open: false }
    }
}

pub struct RepeaterState {
    pub prefs: RepeaterPrefs,
    pub tabs: Vec<RepeaterTab>,
    pub active_id: Option<String>,
    pub search: String,
    pub editor_tab: EditorTab,
    pub response_pane: PaneState,
    pub run_details: RunDetails,
    pub curl_import: CurlImport,
    pub responses: HashMap<String, ShownResponse>,
    pub executing: HashSet<String>,
    pub history: HashMap<String, Vec<RepeaterHistoryItem>>,
    dirty: HashSet<String>,
    last_edit: f64,
}

impl Default for RepeaterState {
    fn default() -> Self {
        Self {
            prefs: RepeaterPrefs::default(),
            tabs: Vec::new(),
            active_id: None,
            search: String::new(),
            editor_tab: EditorTab::Body,
            response_pane: PaneState::default(),
            run_details: RunDetails::default(),
            curl_import: CurlImport::default(),
            responses: HashMap::new(),
            executing: HashSet::new(),
            history: HashMap::new(),
            dirty: HashSet::new(),
            last_edit: 0.0,
        }
    }
}

impl RepeaterState {
    pub fn load(&mut self, db_path: &PathBuf) {
        self.tabs = core_repeater::get_repeater_tabs_db(db_path).unwrap_or_default();
        if let Some(first) = self.tabs.first().map(|t| t.id.clone()) {
            self.select(first, db_path);
        }
    }

    pub fn select(&mut self, id: String, db_path: &PathBuf) {
        self.reload_history(&id, db_path);
        self.active_id = Some(id);
    }

    pub fn mark_dirty(&mut self, id: &str, now: f64) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.updated_at_ms = chrono::Utc::now().timestamp_millis();
        }
        self.dirty.insert(id.to_string());
        self.last_edit = now;
    }

    pub fn flush_if_idle(&mut self, db_path: &PathBuf, now: f64) {
        if !self.dirty.is_empty() && now - self.last_edit >= SAVE_DELAY_SECS {
            self.flush(db_path);
        }
    }

    pub fn flush(&mut self, db_path: &PathBuf) {
        for id in std::mem::take(&mut self.dirty) {
            if let Some(tab) = self.tabs.iter().find(|t| t.id == id) {
                let _ = core_repeater::update_repeater_tab_db(db_path, tab);
            }
        }
    }

    pub fn create(&mut self, tab: RepeaterTab, db_path: &PathBuf) -> String {
        let _ = core_repeater::create_repeater_tab_db(db_path, &tab);
        let id = tab.id.clone();
        self.tabs.insert(0, tab);
        self.search.clear();
        self.select(id.clone(), db_path);
        id
    }

    pub fn create_blank(&mut self, db_path: &PathBuf) {
        let tab = blank_tab(db_path);
        self.create(tab, db_path);
    }

    pub fn create_from_flow(&mut self, flow: &HistoryEntry, detail: Option<&HistoryDetail>, db_path: &PathBuf) {
        let response_headers = detail.map_or(&flow.response_headers, |d| &d.response_headers);
        let response_body = detail.map_or(&flow.response_body, |d| &d.response_body);
        let captured = CapturedRequest::from_flow(flow, detail);
        let headers = captured.headers.clone();
        let body_content = captured.body_content.clone();

        let mut tab = blank_tab(db_path);
        apply_captured(&mut tab, captured);
        let id = self.create(tab, db_path);

        let run = RepeaterHistoryItem {
            id: 0,
            repeater_id: id.clone(),
            method: flow.method.clone(),
            url: flow.url.clone(),
            request_headers: headers,
            request_body: Some(body_content),
            status_code: flow.status_code,
            response_headers: header_items(response_headers),
            response_body: Some(response_body.clone()),
            duration_ms: flow.duration_ms.unwrap_or(0),
            executed_at_ms: chrono::Utc::now().timestamp_millis(),
        };
        if let Ok(history_id) = core_repeater::insert_repeater_history_db(db_path, &run) {
            self.responses.insert(id.clone(), ShownResponse::from_run(&RepeaterHistoryItem { id: history_id, ..run }));
        }
        self.after_run(&id, db_path);
    }

    pub fn create_from_curl(&mut self, curl: ParsedCurl, db_path: &PathBuf) {
        let mut tab = blank_tab(db_path);
        apply_captured(&mut tab, curl.into());
        self.create(tab, db_path);
    }

    pub fn delete(&mut self, id: &str, db_path: &PathBuf) {
        let _ = core_repeater::delete_repeater_tab_db(db_path, id);
        self.tabs.retain(|t| t.id != id);
        for tab in &mut self.tabs {
            let before = tab.pre_requests.len() + tab.post_requests.len();
            tab.pre_requests.retain(|step| step != id);
            tab.post_requests.retain(|step| step != id);
            if tab.pre_requests.len() + tab.post_requests.len() != before {
                self.dirty.insert(tab.id.clone());
            }
        }
        self.responses.remove(id);
        self.history.remove(id);
        self.dirty.remove(id);
        if self.active_id.as_deref() == Some(id) {
            self.active_id = None;
            if let Some(next) = self.tabs.first().map(|t| t.id.clone()) {
                self.select(next, db_path);
            }
        }
    }

    pub fn reload_history(&mut self, id: &str, db_path: &PathBuf) {
        let runs = core_repeater::get_repeater_history_db(db_path, id, 1, 100).unwrap_or_default();
        self.history.insert(id.to_string(), runs);
    }

    pub fn after_run(&mut self, id: &str, db_path: &PathBuf) {
        self.reload_history(id, db_path);
        let Ok(stored) = core_repeater::get_repeater_tab_by_id(db_path, id) else { return };
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.execution_count = stored.execution_count;
            tab.last_status_code = stored.last_status_code;
            tab.last_duration_ms = stored.last_duration_ms;
        }
    }

    pub fn finish_run(&mut self, id: &str, result: Result<RepeaterExecutionResult, String>, db_path: &PathBuf) {
        self.executing.remove(id);
        let shown = match result {
            Ok(result) => ShownResponse::from_result(result),
            Err(error) => ShownResponse {
                history_id: 0,
                status_code: 0,
                status_text: "Request failed".to_string(),
                headers: Vec::new(),
                size: error.len() as u64,
                body: error,
                duration_ms: 0,
            },
        };
        self.responses.insert(id.to_string(), shown);
        self.after_run(id, db_path);
    }

    pub fn show_run(&mut self, tab_id: &str, run_id: i64) {
        if let Some(run) = self.history.get(tab_id).and_then(|runs| runs.iter().find(|r| r.id == run_id)) {
            self.responses.insert(tab_id.to_string(), ShownResponse::from_run(run));
        }
    }

    pub fn load_run(&mut self, tab_id: &str, run_id: i64, now: f64) {
        let Some(run) = self.history.get(tab_id).and_then(|runs| runs.iter().find(|r| r.id() == run_id)).cloned() else {
            return;
        };
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.params = parse_query(&run.url, &tab.params);
            tab.method = run.method.clone();
            tab.url = run.url.clone();
            tab.headers = run.request_headers.clone();
            tab.body_content = run.request_body.clone();
        }
        self.responses.insert(tab_id.to_string(), ShownResponse::from_run(&run));
        self.mark_dirty(tab_id, now);
    }
}

fn apply_captured(tab: &mut RepeaterTab, captured: CapturedRequest) {
    tab.method = captured.method;
    tab.url = captured.url;
    tab.params = captured.params;
    tab.headers = captured.headers;
    tab.body_type = captured.body_type;
    tab.body_content = Some(captured.body_content);
}

fn header(key: &str, value: &str, enabled: bool) -> HeaderItem {
    HeaderItem { id: new_id(), key: key.to_string(), value: value.to_string(), enabled }
}

fn blank_tab(db_path: &PathBuf) -> RepeaterTab {
    let now = chrono::Utc::now().timestamp_millis();
    RepeaterTab {
        id: new_id(),
        method: "GET".to_string(),
        url: "https://httpbin.org/get".to_string(),
        headers: vec![
            header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36", true),
            header("Accept", "*/*", true),
            header("Accept-Language", "en-US,en;q=0.9", true),
            header("Accept-Encoding", "gzip, deflate, br, zstd", true),
            header("Connection", "keep-alive", true),
            header("Sec-Fetch-Dest", "empty", true),
            header("Sec-Fetch-Mode", "cors", true),
            header("Sec-Fetch-Site", "same-site", true),
            header("Content-Type", "application/json", false),
        ],
        params: Vec::new(),
        body_type: "none".to_string(),
        body_content: Some(String::new()),
        extract_rules: Vec::new(),
        pre_requests: Vec::new(),
        post_requests: Vec::new(),
        order_index: core_repeater::top_order_index_db(db_path).unwrap_or(0),
        created_at_ms: now,
        updated_at_ms: now,
        execution_count: 0,
        last_status_code: None,
        last_duration_ms: None,
    }
}
