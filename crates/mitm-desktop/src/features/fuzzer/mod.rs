mod view;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use mitm_core::fuzzer::{
    marker_re, parse_variables, FuzzConfig, FuzzRow, FuzzStart, FuzzTemplate, FuzzVariable, PayloadSet,
    MAX_REQUESTS,
};
use mitm_core::repeater::{HeaderItem, ParamItem};
use crate::features::request_editor::captured::CapturedRequest;
use crate::widgets::tri_state::TriState;

pub use mitm_core::fuzzer::MatchRule;
pub use view::show;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FuzzSortColumn {
    Index,
    Payload,
    Status,
    Size,
    Time,
    Match(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FuzzSearchScope {
    #[default]
    Payload,
    Response,
}

#[derive(Default)]
struct FuzzerCache {
    results_len: usize,
    sort_column: Option<FuzzSortColumn>,
    sort_desc: bool,
    status_filters: HashMap<String, TriState>,
    hide_common_size: bool,
    search_query: String,
    search_scope: FuzzSearchScope,
    response_matches_ver: usize,
    indices: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AttackType {
    #[default]
    Sniper,
    Pitchfork,
    ClusterBomb,
}

impl AttackType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sniper => "sniper",
            Self::Pitchfork => "pitchfork",
            Self::ClusterBomb => "clusterbomb",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Sniper => "Sniper",
            Self::Pitchfork => "Pitchfork",
            Self::ClusterBomb => "Cluster Bomb",
        }
    }

    pub fn tooltip(&self) -> &'static str {
        match self {
            Self::Sniper => "Each variable is fuzzed in turn while the others hold their first payload.",
            Self::Pitchfork => "All variables advance together, stopping at the shortest list.",
            Self::ClusterBomb => "Every combination of the variables' payloads.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadKind {
    List,
    Numbers,
}

/// The editing state for one variable's payloads; `to_set` turns it into a core `PayloadSet`.
pub struct PayloadConfig {
    pub kind: PayloadKind,
    pub list_text: String,
    pub from: f64,
    pub to: f64,
    pub step: f64,
    pub pad: u32,
    pub prefix: String,
    pub suffix: String,
    pub url_encode: bool,
}

impl Default for PayloadConfig {
    fn default() -> Self {
        Self {
            kind: PayloadKind::List,
            list_text: String::new(),
            from: 0.0,
            to: 10.0,
            step: 1.0,
            pad: 0,
            prefix: String::new(),
            suffix: String::new(),
            url_encode: false,
        }
    }
}

impl PayloadConfig {
    fn to_set(&self) -> PayloadSet {
        PayloadSet {
            kind: match self.kind {
                PayloadKind::List => "list",
                PayloadKind::Numbers => "numbers",
            }
            .to_string(),
            list: self.list_text.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect(),
            from: self.from,
            to: self.to,
            step: self.step,
            pad: self.pad,
            url_encode: self.url_encode,
            prefix: self.prefix.clone(),
            suffix: self.suffix.clone(),
        }
    }

    /// The payload count without expanding the whole list, so a huge number range stays cheap.
    pub fn count(&self) -> usize {
        match self.kind {
            PayloadKind::List => self.list_text.lines().filter(|l| !l.trim().is_empty()).count(),
            PayloadKind::Numbers => {
                let step = if self.step == 0.0 { 1.0 } else { self.step };
                let span = self.to - self.from;
                if span != 0.0 && (span > 0.0) != (step > 0.0) {
                    0
                } else {
                    (((span / step).floor() as i64) + 1).clamp(0, MAX_REQUESTS as i64) as usize
                }
            }
        }
    }
}

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
}

impl Default for FuzzerState {
    fn default() -> Self {
        Self {
            prefs: FuzzerPrefs::default(),
            config_tab: ConfigTab::Request,
            request_tab: RequestTab::Params,
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
        if counts.iter().any(|&c| c == 0) {
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

    pub fn toggle_sort(&mut self, col: FuzzSortColumn) {
        if self.sort_column.as_ref() == Some(&col) {
            if !self.sort_desc {
                self.sort_desc = true;
            } else {
                self.sort_column = None;
                self.sort_desc = false;
            }
        } else {
            self.sort_column = Some(col);
            self.sort_desc = false;
        }
    }

    pub fn has_active_filters(&self) -> bool {
        self.hide_common_size
            || !self.search_query.trim().is_empty()
            || self.status_filters.values().any(|s| *s != TriState::Neutral)
    }

    pub fn reset_filters(&mut self) {
        self.hide_common_size = false;
        self.status_filters.clear();
        self.search_query.clear();
        self.response_matches = None;
        self.pending_search_key = None;
        self.response_matches_ver += 1;
    }

    pub fn visible_indices(&mut self) -> &[usize] {
        if self.cache.results_len == self.results.len()
            && self.cache.sort_column == self.sort_column
            && self.cache.sort_desc == self.sort_desc
            && self.cache.status_filters == self.status_filters
            && self.cache.hide_common_size == self.hide_common_size
            && self.cache.search_query == self.search_query
            && self.cache.search_scope == self.search_scope
            && self.cache.response_matches_ver == self.response_matches_ver
        {
            return &self.cache.indices;
        }

        let indices = self.compute_filtered_indices();
        self.cache = FuzzerCache {
            results_len: self.results.len(),
            sort_column: self.sort_column.clone(),
            sort_desc: self.sort_desc,
            status_filters: self.status_filters.clone(),
            hide_common_size: self.hide_common_size,
            search_query: self.search_query.clone(),
            search_scope: self.search_scope,
            response_matches_ver: self.response_matches_ver,
            indices,
        };
        &self.cache.indices
    }

    fn compute_filtered_indices(&self) -> Vec<usize> {
        let has_includes = self.status_filters.values().any(|&s| s == TriState::Include);
        let common_size = if self.hide_common_size {
            most_frequent_size(&self.results)
        } else {
            None
        };
        let query = self.search_query.trim().to_lowercase();
        let has_search = !query.is_empty();

        let mut indices: Vec<usize> = (0..self.results.len())
            .filter(|&i| {
                let r = &self.results[i];
                if let Some(common) = common_size {
                    if r.error.is_none() && r.status_code != 0 && r.response_size == common {
                        return false;
                    }
                }
                let group = status_group(r.status_code);
                let status_matches = match self.status_filters.get(group).copied().unwrap_or(TriState::Neutral) {
                    TriState::Exclude => false,
                    TriState::Include => true,
                    TriState::Neutral => !has_includes,
                };
                if !status_matches {
                    return false;
                }
                if has_search {
                    match self.search_scope {
                        FuzzSearchScope::Payload => {
                            if !r.payloads.iter().any(|p| p.to_lowercase().contains(&query)) {
                                return false;
                            }
                        }
                        FuzzSearchScope::Response => {
                            if let Some(ref matches) = self.response_matches {
                                if !matches.contains(&r.idx) {
                                    return false;
                                }
                            }
                        }
                    }
                }
                true
            })
            .collect();

        if let Some(ref col) = self.sort_column {
            let desc = self.sort_desc;
            let rule_idx = match col {
                FuzzSortColumn::Match(name) => self.match_rules.iter().position(|r| &r.name == name),
                _ => None,
            };

            indices.sort_by(|&a, &b| {
                let ra = &self.results[a];
                let rb = &self.results[b];
                let ord = match col {
                    FuzzSortColumn::Index => ra.idx.cmp(&rb.idx),
                    FuzzSortColumn::Payload => ra.payloads.cmp(&rb.payloads),
                    FuzzSortColumn::Status => ra.status_code.cmp(&rb.status_code),
                    FuzzSortColumn::Size => ra.response_size.cmp(&rb.response_size),
                    FuzzSortColumn::Time => ra.duration_ms.cmp(&rb.duration_ms),
                    FuzzSortColumn::Match(_) => {
                        let va = rule_idx.and_then(|idx| ra.matches.get(idx)).map(String::as_str).unwrap_or("");
                        let vb = rule_idx.and_then(|idx| rb.matches.get(idx)).map(String::as_str).unwrap_or("");
                        match (va.parse::<i64>(), vb.parse::<i64>()) {
                            (Ok(na), Ok(nb)) => na.cmp(&nb),
                            _ => va.cmp(vb),
                        }
                    }
                };
                if desc {
                    ord.reverse()
                } else {
                    ord
                }
            });
        }

        indices
    }

    pub fn select_result(&mut self, run_id: i64, idx: u32, db_path: &std::path::PathBuf) {
        if self.selected_idx == Some(idx) && self.selected_result.is_some() {
            return;
        }
        self.selected_idx = Some(idx);
        self.selected_result = mitm_core::fuzzer::get_result_db(db_path, run_id, idx).ok().flatten();
    }

    pub fn sync_response_search(
        &mut self,
        rt: &tokio::runtime::Runtime,
        ctx: &eframe::egui::Context,
        core: &std::sync::Arc<mitm_core::MitmCore>,
        jobs: &crate::jobs::Jobs<(i64, String, Vec<u32>)>,
    ) {
        let query = self.search_query.trim();
        if self.search_scope != FuzzSearchScope::Response || query.is_empty() {
            if self.response_matches.is_some() || self.pending_search_key.is_some() {
                self.response_matches = None;
                self.pending_search_key = None;
                self.response_matches_ver += 1;
            }
            return;
        }
        let Some(run_id) = self.run_id else {
            if self.response_matches.is_some() || self.pending_search_key.is_some() {
                self.response_matches = None;
                self.pending_search_key = None;
                self.response_matches_ver += 1;
            }
            return;
        };
        let key = (run_id, query.to_string());
        if self.pending_search_key.as_ref() == Some(&key) {
            return;
        }
        self.pending_search_key = Some(key.clone());
        let core = std::sync::Arc::clone(core);
        let q = query.to_string();
        jobs.spawn(rt, ctx, async move {
            let matches = core.search_fuzz_responses(run_id, &q).unwrap_or_default();
            (run_id, q, matches)
        });
    }

    pub fn set_response_search_results(&mut self, run_id: i64, query: &str, matches: Vec<u32>) {
        let current_q = self.search_query.trim();
        if self.run_id == Some(run_id) && self.search_scope == FuzzSearchScope::Response && current_q == query {
            self.response_matches = Some(matches.into_iter().collect());
            self.response_matches_ver += 1;
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

pub fn status_group(code: u16) -> &'static str {
    match code {
        0 => "ERR",
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        500..=599 => "5xx",
        _ => "ERR",
    }
}

pub fn most_frequent_size(rows: &[FuzzRow]) -> Option<u64> {
    let mut counts: HashMap<u64, usize> = HashMap::new();
    let mut best: Option<u64> = None;
    let mut best_count = 0;
    for row in rows {
        if row.error.is_some() || row.status_code == 0 {
            continue;
        }
        let count = counts.entry(row.response_size).and_modify(|c| *c += 1).or_insert(1);
        if *count > best_count {
            best = Some(row.response_size);
            best_count = *count;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut state = FuzzerState::default();
        state.url = "https://httpbin.org/status/{{missing}}".to_string();
        assert_eq!(state.request_estimate(), 0);

        state.payloads.insert("missing".to_string(), PayloadConfig::default());
        assert_eq!(state.request_estimate(), 0);

        state.url = "https://httpbin.org/status/200".to_string();
        assert_eq!(state.request_estimate(), 0);
    }

    #[test]
    fn options_populate_config() {
        let mut state = FuzzerState::default();
        state.concurrency = 12;
        state.delay_ms = 250;
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
        let mut state = FuzzerState::default();
        state.selected_idx = Some(5);
        state.selected_result = Some(mitm_core::fuzzer::FuzzResult {
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
        });
        assert_eq!(state.selected_idx, Some(5));
        assert!(state.selected_result.is_some());

        state.clear();
        assert_eq!(state.selected_idx, None);
        assert!(state.selected_result.is_none());
    }

    #[test]
    fn search_payload_and_response() {
        let mut state = FuzzerState::default();
        state.run_id = Some(42);
        state.results = vec![
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
        ];

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
