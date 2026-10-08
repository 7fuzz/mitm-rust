mod view;

use serde::{Deserialize, Serialize};
use mitm_core::fuzzer::{marker_re, parse_variables, FuzzConfig, FuzzRow, FuzzStart, FuzzTemplate, FuzzVariable, PayloadSet};
use mitm_core::repeater::{HeaderItem, ParamItem};
use crate::features::request_editor::captured::CapturedRequest;

pub use view::show;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Idle,
    Running,
    Stopped,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigTab {
    Request,
    Payloads,
    Options,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestTab {
    Params,
    Headers,
    Body,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FuzzerPrefs {
    pub split: f32,
}

impl Default for FuzzerPrefs {
    fn default() -> Self {
        Self { split: 0.45 }
    }
}

pub struct FuzzerState {
    pub prefs: FuzzerPrefs,
    pub config_tab: ConfigTab,
    pub request_tab: RequestTab,
    pub method: String,
    pub url: String,
    pub params: Vec<ParamItem>,
    pub headers: Vec<HeaderItem>,
    pub body_type: String,
    pub body_content: String,
    pub wordlist_input: String,
    pub concurrency: usize,
    pub status: RunStatus,
    pub run_id: Option<i64>,
    pub total: usize,
    pub results: Vec<FuzzRow>,
    pub error: Option<String>,
}

impl Default for FuzzerState {
    fn default() -> Self {
        Self {
            prefs: FuzzerPrefs::default(),
            config_tab: ConfigTab::Request,
            request_tab: RequestTab::Params,
            method: "GET".to_string(),
            url: "https://httpbin.org/status/{{status}}".to_string(),
            params: Vec::new(),
            headers: vec![HeaderItem {
                id: uuid::Uuid::new_v4().to_string(),
                key: "User-Agent".to_string(),
                value: "MITM-Native/0.2".to_string(),
                enabled: true,
            }],
            body_type: "none".to_string(),
            body_content: String::new(),
            wordlist_input: "200\n301\n400\n404\n500".to_string(),
            concurrency: 5,
            status: RunStatus::Idle,
            run_id: None,
            total: 0,
            results: Vec::new(),
            error: None,
        }
    }
}

impl FuzzerState {
    pub fn load_capture(&mut self, captured: CapturedRequest) {
        self.method = captured.method;
        self.url = captured.url;
        self.params = captured.params;
        self.headers = captured.headers;
        self.body_type = captured.body_type;
        self.body_content = captured.body_content;
    }

    /// Strips every `{{name}}` back to `name`, reversing the Mark button across the request.
    pub fn clear_marks(&mut self) {
        let strip = |s: &str| marker_re().replace_all(s, "$1").into_owned();
        self.url = strip(&self.url);
        for p in &mut self.params {
            p.value = strip(&p.value);
        }
        for h in &mut self.headers {
            h.value = strip(&h.value);
        }
        self.body_content = strip(&self.body_content);
    }

    /// Every marker draws from the one wordlist in lockstep, so all markers get the same payload per request.
    pub fn config(&self) -> FuzzConfig {
        let template = FuzzTemplate {
            method: self.method.clone(),
            url: self.url.clone(),
            headers: self.headers.clone(),
            params: self.params.clone(),
            body_type: self.body_type.clone(),
            body: (self.body_type != "none" && !self.body_content.is_empty()).then(|| self.body_content.clone()),
        };
        let list: Vec<String> =
            self.wordlist_input.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect();
        let variables = parse_variables(&template)
            .into_iter()
            .map(|name| FuzzVariable {
                name,
                set: PayloadSet {
                    kind: "list".to_string(),
                    list: list.clone(),
                    from: 0.0,
                    to: 0.0,
                    step: 1.0,
                    pad: 0,
                    url_encode: false,
                    prefix: String::new(),
                    suffix: String::new(),
                },
            })
            .collect();
        FuzzConfig {
            template,
            attack_type: "pitchfork".to_string(),
            variables,
            match_rules: Vec::new(),
            concurrency: self.concurrency as u32,
            delay_ms: 0,
        }
    }

    pub fn started(&mut self, start: Result<FuzzStart, String>) {
        match start {
            Ok(start) => {
                self.status = RunStatus::Running;
                self.run_id = Some(start.run_id);
                self.total = start.total;
                self.results = Vec::with_capacity(start.total);
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
    }

    pub fn push_row(&mut self, run_id: i64, row: FuzzRow) {
        if self.run_id == Some(run_id) {
            self.results.push(row);
        }
    }

    /// `stored` reloads the rows when some row events were missed.
    pub fn finished(&mut self, run_id: i64, completed: usize, stopped: bool, stored: impl FnOnce() -> Vec<FuzzRow>) {
        if self.run_id != Some(run_id) {
            return;
        }
        self.status = if stopped { RunStatus::Stopped } else { RunStatus::Done };
        if self.results.len() != completed {
            self.results = stored();
        }
    }

    pub fn clear(&mut self) {
        self.status = RunStatus::Idle;
        self.run_id = None;
        self.total = 0;
        self.results = Vec::new();
    }
}
