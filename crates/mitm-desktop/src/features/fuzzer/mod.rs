mod commands;
mod payloads;
mod results;
mod view;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use mitm_core::fuzzer::{
    marker_re, parse_variables, FuzzConfig, FuzzRow, FuzzStart, FuzzTemplate, FuzzVariable,
};
use mitm_core::repeater::{HeaderItem, ParamItem};
use crate::features::request_editor::captured::CapturedRequest;
use crate::widgets::tri_state::TriState;

pub use mitm_core::fuzzer::MatchRule;
pub use commands::{handle, poll, Cmd};
pub use payloads::{AttackType, PayloadConfig, PayloadKind};
pub use results::{FuzzSearchScope, FuzzSortColumn};
use results::FuzzerCache;
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
    Match,
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
    pub detail_split: f32,
}

impl Default for FuzzerPrefs {
    fn default() -> Self {
        Self { split: 0.45, detail_split: 0.55 }
    }
}

/// The request and attack setup, saved across restarts. Runs live in the DB instead.
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct FuzzerConfig {
    pub attack_type: AttackType,
    pub method: String,
    pub url: String,
    pub params: Vec<ParamItem>,
    pub headers: Vec<HeaderItem>,
    pub body_type: String,
    pub body_content: String,
    pub payloads: HashMap<String, PayloadConfig>,
    pub match_rules: Vec<MatchRule>,
    pub concurrency: usize,
    pub delay_ms: u64,
}

impl Default for FuzzerConfig {
    fn default() -> Self {
        Self {
            attack_type: AttackType::Sniper,
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
            payloads: HashMap::from([(
                "status".to_string(),
                PayloadConfig { list_text: "200\n301\n400\n404\n500".to_string(), ..PayloadConfig::default() },
            )]),
            match_rules: Vec::new(),
            concurrency: 5,
            delay_ms: 0,
        }
    }
}

pub struct FuzzerState {
    pub prefs: FuzzerPrefs,
    pub config_tab: ConfigTab,
    pub request_tab: RequestTab,
    pub attack_type: AttackType,
    pub method: String,
    pub url: String,
    pub params: Vec<ParamItem>,
    pub headers: Vec<HeaderItem>,
    pub body_type: String,
    pub body_content: String,
    pub payloads: HashMap<String, PayloadConfig>,
    pub match_rules: Vec<MatchRule>,
    pub concurrency: usize,
    pub delay_ms: u64,
    pub sort_column: Option<FuzzSortColumn>,
    pub sort_desc: bool,
    pub status_filters: HashMap<String, TriState>,
    pub hide_common_size: bool,
    cache: FuzzerCache,
    pub status: RunStatus,
    pub run_id: Option<i64>,
    pub total: usize,
    pub results: Vec<FuzzRow>,
    pub selected_idx: Option<u32>,
    pub selected_result: Option<mitm_core::fuzzer::FuzzResult>,
    pub response_pane: crate::widgets::message_pane::PaneState,
    pub search_query: String,
    pub search_scope: FuzzSearchScope,
    pub response_matches: Option<std::collections::HashSet<u32>>,
    pub pending_search_key: Option<(i64, String)>,
    response_matches_ver: usize,
    pub is_saved: bool,
    pub run_name: Option<String>,
    pub saving_run: bool,
    pub focus_save_input: bool,
    pub save_name_input: String,
    pub saved_runs: Vec<mitm_core::fuzzer::FuzzRunMeta>,
    pub error: Option<String>,
    pub starting: bool,
    jobs: commands::FuzzJobs,
}

impl Default for FuzzerState {
    fn default() -> Self {
        let config = FuzzerConfig::default();
        Self {
            prefs: FuzzerPrefs::default(),
            config_tab: ConfigTab::Request,
            request_tab: RequestTab::Params,
            attack_type: config.attack_type,
            method: config.method,
            url: config.url,
            params: config.params,
            headers: config.headers,
            body_type: config.body_type,
            body_content: config.body_content,
            payloads: config.payloads,
            match_rules: config.match_rules,
            concurrency: config.concurrency,
            delay_ms: config.delay_ms,
            sort_column: None,
            sort_desc: false,
            status_filters: HashMap::new(),
            hide_common_size: false,
            cache: FuzzerCache::default(),
            status: RunStatus::Idle,
            run_id: None,
            total: 0,
            results: Vec::new(),
            selected_idx: None,
            selected_result: None,
            response_pane: crate::widgets::message_pane::PaneState::default(),
            search_query: String::new(),
            search_scope: FuzzSearchScope::Payload,
            response_matches: None,
            pending_search_key: None,
            response_matches_ver: 0,
            is_saved: false,
            run_name: None,
            saving_run: false,
            focus_save_input: false,
            save_name_input: String::new(),
            saved_runs: Vec::new(),
            error: None,
            starting: false,
            jobs: Default::default(),
        }
    }
}

impl FuzzerState {
    pub fn snapshot(&self) -> FuzzerConfig {
        FuzzerConfig {
            attack_type: self.attack_type,
            method: self.method.clone(),
            url: self.url.clone(),
            params: self.params.clone(),
            headers: self.headers.clone(),
            body_type: self.body_type.clone(),
            body_content: self.body_content.clone(),
            payloads: self.payloads.clone(),
            match_rules: self.match_rules.clone(),
            concurrency: self.concurrency,
            delay_ms: self.delay_ms,
        }
    }

    pub fn restore(&mut self, config: FuzzerConfig) {
        self.attack_type = config.attack_type;
        self.method = config.method;
        self.url = config.url;
        self.params = config.params;
        self.headers = config.headers;
        self.body_type = config.body_type;
        self.body_content = config.body_content;
        self.payloads = config.payloads;
        self.match_rules = config.match_rules;
        self.concurrency = config.concurrency;
        self.delay_ms = config.delay_ms;
    }

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

    pub fn template(&self) -> FuzzTemplate {
        FuzzTemplate {
            method: self.method.clone(),
            url: self.url.clone(),
            headers: self.headers.clone(),
            params: self.params.clone(),
            body_type: self.body_type.clone(),
            body: (self.body_type != "none" && !self.body_content.is_empty()).then(|| self.body_content.clone()),
        }
    }

    pub fn variable_names(&self) -> Vec<String> {
        parse_variables(&self.template())
    }

    pub fn config(&self) -> FuzzConfig {
        let template = self.template();
        let variables = parse_variables(&template)
            .into_iter()
            .map(|name| {
                let set = self.payloads.get(&name).map_or_else(|| PayloadConfig::default().to_set(), PayloadConfig::to_set);
                FuzzVariable { name, set }
            })
            .collect();
        FuzzConfig {
            template,
            attack_type: self.attack_type.as_str().to_string(),
            variables,
            match_rules: self.match_rules.clone(),
            concurrency: self.concurrency as u32,
            delay_ms: self.delay_ms,
        }
    }

    pub fn request_estimate(&self) -> usize {
        let names = self.variable_names();
        if names.is_empty() {
            return 0;
        }
        let counts: Vec<usize> = names
            .iter()
            .map(|name| self.payloads.get(name).map_or(0, |cfg| cfg.count()))
            .collect();
        if counts.contains(&0) {
            return 0;
        }
        match self.attack_type {
            AttackType::Sniper => counts.iter().sum(),
            AttackType::Pitchfork => counts.into_iter().min().unwrap_or(0),
            AttackType::ClusterBomb => {
                counts.into_iter().try_fold(1usize, |acc, c| acc.checked_mul(c)).unwrap_or(usize::MAX)
            }
        }
    }

    pub fn started(&mut self, start: Result<FuzzStart, String>) {
        match start {
            Ok(start) => {
                self.status = RunStatus::Running;
                self.run_id = Some(start.run_id);
                self.run_name = None;
                self.is_saved = false;
                self.saving_run = false;
                self.save_name_input.clear();
                self.total = start.total;
                self.results = Vec::with_capacity(start.total);
                self.selected_idx = None;
                self.selected_result = None;
                self.pending_search_key = None;
                self.response_matches = None;
                self.response_matches_ver += 1;
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
        self.run_name = None;
        self.is_saved = false;
        self.saving_run = false;
        self.save_name_input.clear();
        self.total = 0;
        self.results = Vec::new();
        self.selected_idx = None;
        self.selected_result = None;
        self.pending_search_key = None;
        self.response_matches = None;
        self.response_matches_ver += 1;
        self.cache = FuzzerCache::default();
    }

    pub fn load_saved_runs(&mut self, db_path: &std::path::PathBuf) {
        if let Ok(runs) = mitm_core::fuzzer::list_runs_db(db_path) {
            self.saved_runs = runs;
        }
    }

    pub fn load_run(&mut self, loaded: mitm_core::fuzzer::LoadedFuzzRun) {
        self.run_id = Some(loaded.id);
        self.run_name = Some(loaded.name);
        self.is_saved = true;
        self.saving_run = false;
        self.save_name_input.clear();
        self.status = RunStatus::Done;
        self.total = loaded.total as usize;
        self.results = loaded.rows;
        self.attack_type = match loaded.attack_type.as_str() {
            "pitchfork" => AttackType::Pitchfork,
            "clusterbomb" => AttackType::ClusterBomb,
            _ => AttackType::Sniper,
        };
        self.method = loaded.config.template.method;
        self.url = loaded.config.template.url;
        self.params = loaded.config.template.params;
        self.headers = loaded.config.template.headers;
        self.body_type = loaded.config.template.body_type;
        self.body_content = loaded.config.template.body.unwrap_or_default();
        self.payloads.clear();
        for v in loaded.config.variables {
            let kind = match v.set.kind.as_str() {
                "numbers" => PayloadKind::Numbers,
                _ => PayloadKind::List,
            };
            let list_text = v.set.list.join("\n");
            self.payloads.insert(
                v.name,
                PayloadConfig {
                    kind,
                    list_text,
                    from: v.set.from,
                    to: v.set.to,
                    step: v.set.step,
                    pad: v.set.pad,
                    prefix: v.set.prefix,
                    suffix: v.set.suffix,
                    url_encode: v.set.url_encode,
                },
            );
        }
        self.match_rules = loaded.config.match_rules;
        self.concurrency = loaded.config.concurrency as usize;
        self.delay_ms = loaded.config.delay_ms;
        self.selected_idx = None;
        self.selected_result = None;
        self.reset_filters();
        self.error = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mitm_core::fuzzer::PayloadSet;

    #[test]
    fn config_survives_a_save_and_restore() {
        let mut state = FuzzerState {
            url: "https://example.test/{{id}}".to_string(),
            attack_type: AttackType::ClusterBomb,
            delay_ms: 250,
            ..FuzzerState::default()
        };
        state.payloads.insert("id".into(), PayloadConfig { kind: PayloadKind::Numbers, to: 99.0, ..PayloadConfig::default() });
        let saved = serde_json::to_string(&state.snapshot()).unwrap();

        let mut restored = FuzzerState::default();
        restored.restore(serde_json::from_str(&saved).unwrap());
        assert_eq!(restored.url, state.url);
        assert_eq!(restored.attack_type, AttackType::ClusterBomb);
        assert_eq!(restored.delay_ms, 250);
        assert_eq!(restored.payloads["id"].to, 99.0);
        assert_eq!(restored.payloads["status"].list_text, state.payloads["status"].list_text);
    }

    #[test]
    fn attack_types_estimate_requests() {
        let mut state = FuzzerState::default();
        assert_eq!(state.attack_type, AttackType::Sniper);
        assert_eq!(state.request_estimate(), 5);

        state.attack_type = AttackType::Pitchfork;
        assert_eq!(state.request_estimate(), 5);

        state.attack_type = AttackType::ClusterBomb;
        assert_eq!(state.request_estimate(), 5);

        state.url = "https://httpbin.org/status/{{status}}?id={{id}}".to_string();
        state.payloads.insert(
            "id".to_string(),
            PayloadConfig {
                list_text: "1\n2\n3".to_string(),
                ..Default::default()
            },
        );

        state.attack_type = AttackType::Sniper;
        assert_eq!(state.request_estimate(), 8);

        state.attack_type = AttackType::Pitchfork;
        assert_eq!(state.request_estimate(), 3);

        state.attack_type = AttackType::ClusterBomb;
        assert_eq!(state.request_estimate(), 15);
    }

    #[test]
    fn empty_or_missing_payloads_estimate_zero() {
        let mut state = FuzzerState {
            url: "https://httpbin.org/status/{{missing}}".to_string(),
            ..FuzzerState::default()
        };
        assert_eq!(state.request_estimate(), 0);

        state.payloads.insert("missing".to_string(), PayloadConfig::default());
        assert_eq!(state.request_estimate(), 0);

        state.url = "https://httpbin.org/status/200".to_string();
        assert_eq!(state.request_estimate(), 0);
    }

    #[test]
    fn options_populate_config() {
        let state = FuzzerState {
            concurrency: 12,
            delay_ms: 250,
            ..FuzzerState::default()
        };
        let cfg = state.config();
        assert_eq!(cfg.concurrency, 12);
        assert_eq!(cfg.delay_ms, 250);
    }

    #[test]
    fn match_rules_populate_config() {
        let mut state = FuzzerState::default();
        state.match_rules.push(MatchRule {
            name: "err".to_string(),
            kind: "contains".to_string(),
            pattern: "500".to_string(),
        });
        let cfg = state.config();
        assert_eq!(cfg.match_rules.len(), 1);
        assert_eq!(cfg.match_rules[0].name, "err");
        assert_eq!(cfg.match_rules[0].pattern, "500");
    }

    #[test]
    fn filtering_and_sorting_results() {
        let mut state = FuzzerState::default();
        state.match_rules.push(MatchRule {
            name: "rule1".to_string(),
            kind: "contains".to_string(),
            pattern: "x".to_string(),
        });
        state.results = vec![
            FuzzRow {
                idx: 0,
                payloads: vec!["b".to_string()],
                status_code: 200,
                response_size: 100,
                duration_ms: 50,
                error: None,
                matches: vec!["5".to_string()],
            },
            FuzzRow {
                idx: 1,
                payloads: vec!["a".to_string()],
                status_code: 404,
                response_size: 200,
                duration_ms: 10,
                error: None,
                matches: vec!["15".to_string()],
            },
            FuzzRow {
                idx: 2,
                payloads: vec!["c".to_string()],
                status_code: 0,
                response_size: 0,
                duration_ms: 0,
                error: Some("timeout".to_string()),
                matches: vec!["0".to_string()],
            },
            FuzzRow {
                idx: 3,
                payloads: vec!["d".to_string()],
                status_code: 200,
                response_size: 100,
                duration_ms: 30,
                error: None,
                matches: vec!["2".to_string()],
            },
        ];

        assert_eq!(state.visible_indices(), &[0, 1, 2, 3]);

        state.toggle_sort(FuzzSortColumn::Payload);
        assert_eq!(state.visible_indices(), &[1, 0, 2, 3]);

        state.toggle_sort(FuzzSortColumn::Payload);
        assert_eq!(state.visible_indices(), &[3, 2, 0, 1]);

        state.toggle_sort(FuzzSortColumn::Match("rule1".to_string()));
        assert_eq!(state.visible_indices(), &[2, 3, 0, 1]);

        state.toggle_sort(FuzzSortColumn::Match("rule1".to_string()));
        state.toggle_sort(FuzzSortColumn::Match("rule1".to_string()));

        state.hide_common_size = true;
        assert_eq!(state.visible_indices(), &[1, 2]);

        state.hide_common_size = false;
        state.status_filters.insert("4xx".to_string(), TriState::Include);
        assert_eq!(state.visible_indices(), &[1]);

        state.reset_filters();
        state.status_filters.insert("2xx".to_string(), TriState::Exclude);
        assert_eq!(state.visible_indices(), &[1, 2]);
    }

    #[test]
    fn select_and_clear_result() {
        let mut state = FuzzerState {
            selected_idx: Some(5),
            selected_result: Some(mitm_core::fuzzer::FuzzResult {
                idx: 5,
                payloads: vec!["foo".to_string()],
                method: "GET".to_string(),
                url: "https://example.com/foo".to_string(),
                request_headers: vec![],
                request_body: None,
                status_code: 200,
                response_headers: vec![],
                response_body: Some("ok".to_string()),
                response_size: 2,
                duration_ms: 10,
                error: None,
                matches: vec![],
            }),
            ..FuzzerState::default()
        };
        assert_eq!(state.selected_idx, Some(5));
        assert!(state.selected_result.is_some());

        state.clear();
        assert_eq!(state.selected_idx, None);
        assert!(state.selected_result.is_none());
    }

    #[test]
    fn search_payload_and_response() {
        let mut state = FuzzerState {
            run_id: Some(42),
            results: vec![
                FuzzRow {
                    idx: 0,
                    payloads: vec!["apple".to_string()],
                    status_code: 200,
                    response_size: 100,
                    duration_ms: 10,
                    error: None,
                    matches: vec![],
                },
                FuzzRow {
                    idx: 1,
                    payloads: vec!["banana".to_string()],
                    status_code: 200,
                    response_size: 100,
                    duration_ms: 10,
                    error: None,
                    matches: vec![],
                },
                FuzzRow {
                    idx: 2,
                    payloads: vec!["cherry".to_string()],
                    status_code: 200,
                    response_size: 100,
                    duration_ms: 10,
                    error: None,
                    matches: vec![],
                },
            ],
            ..FuzzerState::default()
        };

        state.search_scope = FuzzSearchScope::Payload;
        state.search_query = "nan".to_string();
        assert_eq!(state.visible_indices(), &[1]);

        state.search_query.clear();
        assert_eq!(state.visible_indices(), &[0, 1, 2]);

        state.search_scope = FuzzSearchScope::Response;
        state.search_query = "secret".to_string();
        state.set_response_search_results(42, "secret", vec![0, 2]);
        assert_eq!(state.visible_indices(), &[0, 2]);

        state.reset_filters();
        assert_eq!(state.visible_indices(), &[0, 1, 2]);
        assert_eq!(state.search_query, "");
    }

    #[test]
    fn load_run_restores_state_and_config() {
        let mut state = FuzzerState::default();
        let loaded = mitm_core::fuzzer::LoadedFuzzRun {
            id: 99,
            name: "Test Run".to_string(),
            attack_type: "pitchfork".to_string(),
            total: 1,
            config: FuzzConfig {
                template: mitm_core::fuzzer::FuzzTemplate {
                    method: "POST".to_string(),
                    url: "http://example.com/api/{{id}}".to_string(),
                    headers: vec![],
                    params: vec![],
                    body_type: "raw".to_string(),
                    body: Some("payload".to_string()),
                },
                attack_type: "pitchfork".to_string(),
                variables: vec![FuzzVariable {
                    name: "id".to_string(),
                    set: PayloadSet {
                        kind: "list".to_string(),
                        list: vec!["foo".to_string()],
                        from: 0.0,
                        to: 0.0,
                        step: 0.0,
                        pad: 0,
                        url_encode: false,
                        prefix: String::new(),
                        suffix: String::new(),
                    },
                }],
                match_rules: vec![],
                concurrency: 12,
                delay_ms: 50,
            },
            rows: vec![FuzzRow {
                idx: 0,
                payloads: vec!["foo".to_string()],
                status_code: 200,
                response_size: 42,
                duration_ms: 15,
                error: None,
                matches: vec![],
            }],
        };

        state.load_run(loaded);
        assert_eq!(state.run_id, Some(99));
        assert_eq!(state.run_name.as_deref(), Some("Test Run"));
        assert!(state.is_saved);
        assert_eq!(state.attack_type, AttackType::Pitchfork);
        assert_eq!(state.method, "POST");
        assert_eq!(state.url, "http://example.com/api/{{id}}");
        assert_eq!(state.concurrency, 12);
        assert_eq!(state.delay_ms, 50);
        assert_eq!(state.results.len(), 1);
        assert_eq!(state.status, RunStatus::Done);

        state.clear();
        assert_eq!(state.run_id, None);
        assert_eq!(state.run_name, None);
        assert!(!state.is_saved);
        assert_eq!(state.status, RunStatus::Idle);
    }
}
