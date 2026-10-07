mod view;

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::path::PathBuf;
use mitm_core::state::{HistoryDetail, HistoryEntry, ListenerConfig};
use crate::widgets::tri_state::TriState;

pub use view::show;

pub const METHODS: [&str; 7] = ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"];
pub const STATUS_GROUPS: [&str; 5] = ["2xx", "3xx", "4xx", "5xx", "ERR"];
const ALL_SOURCES: &str = "ALL";

pub struct TrafficState {
    pub flows: VecDeque<HistoryEntry>,
    pub max_flows: usize,
    pub selected_id: Option<i64>,
    pub selected_detail: Option<HistoryDetail>,
    pub search_query: String,
    pub source_filter: String,
    pub method_filters: HashMap<String, TriState>,
    pub status_filters: HashMap<String, TriState>,
    pub auto_scroll: bool,
}

impl Default for TrafficState {
    fn default() -> Self {
        Self {
            flows: VecDeque::with_capacity(1000),
            max_flows: 10_000,
            selected_id: None,
            selected_detail: None,
            search_query: String::new(),
            source_filter: ALL_SOURCES.to_string(),
            method_filters: METHODS.iter().map(|m| (m.to_string(), TriState::Neutral)).collect(),
            status_filters: STATUS_GROUPS.iter().map(|s| (s.to_string(), TriState::Neutral)).collect(),
            auto_scroll: true,
        }
    }
}

impl TrafficState {
    pub fn load_from_db(&mut self, db_path: &PathBuf) {
        if let Ok(entries) = mitm_core::db::get_history_logs(db_path, 500, 0, None, None, None, false) {
            self.flows = entries.into();
        }
    }

    pub fn upsert(&mut self, entry: HistoryEntry) {
        if let Some(existing) = self.flows.iter_mut().find(|f| f.id == entry.id) {
            existing.status_code = entry.status_code;
            existing.response_size = entry.response_size;
            existing.duration_ms = entry.duration_ms;
            existing.content_type = entry.content_type;
            existing.phase = entry.phase;
            existing.is_intercepted = entry.is_intercepted;
            existing.is_rewritten = entry.is_rewritten;
            existing.is_failed = entry.is_failed;
            if !entry.response_headers.is_empty() {
                existing.response_headers = entry.response_headers;
            }
            if !entry.response_body.is_empty() {
                existing.response_body = entry.response_body;
            }
            if entry.response_at.is_some() {
                existing.response_at = entry.response_at;
            }
            return;
        }

        if self.flows.len() >= self.max_flows {
            self.flows.pop_back();
        }
        self.flows.push_front(entry);
    }

    pub fn update_response(&mut self, id: i64, status: u16, size: u64, duration_ms: u64) {
        if let Some(flow) = self.flows.iter_mut().find(|f| f.id == id) {
            flow.status_code = status;
            flow.response_size = size;
            flow.duration_ms = Some(duration_ms);
            flow.phase = "response".to_string();
        }
    }

    pub fn clear(&mut self) {
        self.flows.clear();
        self.selected_id = None;
        self.selected_detail = None;
    }

    pub fn select(&mut self, id: i64, db_path: &PathBuf) {
        self.selected_id = Some(id);
        self.selected_detail = load_detail(db_path, id);
    }

    pub fn refresh_detail_if_selected(&mut self, id: i64, db_path: &PathBuf) {
        if self.selected_id == Some(id) {
            self.selected_detail = load_detail(db_path, id);
        }
    }

    pub fn flow(&self, id: i64) -> Option<&HistoryEntry> {
        self.flows.iter().find(|f| f.id == id)
    }

    pub fn selected_flow(&self) -> Option<&HistoryEntry> {
        self.selected_id.and_then(|id| self.flow(id))
    }

    pub fn has_active_filters(&self) -> bool {
        self.source_filter != ALL_SOURCES
            || self.method_filters.values().any(|&v| v != TriState::Neutral)
            || self.status_filters.values().any(|&v| v != TriState::Neutral)
    }

    pub fn reset_filters(&mut self) {
        self.search_query.clear();
        self.source_filter = ALL_SOURCES.to_string();
        self.method_filters.values_mut().for_each(|v| *v = TriState::Neutral);
        self.status_filters.values_mut().for_each(|v| *v = TriState::Neutral);
    }

    pub fn source_labels(&self, listeners: &[ListenerConfig]) -> Vec<String> {
        let from_listeners = listeners.iter().map(|l| &l.label);
        let from_flows = self.flows.iter().map(|f| &f.listener_label);
        from_listeners
            .chain(from_flows)
            .filter(|l| !l.is_empty())
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn filtered_indices(&self) -> Vec<usize> {
        let search = self.search_query.trim().to_lowercase();
        let has_included_methods = self.method_filters.values().any(|&s| s == TriState::Include);
        let has_included_status = self.status_filters.values().any(|&s| s == TriState::Include);

        self.flows
            .iter()
            .enumerate()
            .filter(|(_, flow)| {
                self.matches_source(flow)
                    && passes_tri_state(&self.method_filters, &flow.method.to_uppercase(), has_included_methods)
                    && passes_tri_state(&self.status_filters, status_group(flow.status_code), has_included_status)
                    && matches_search(flow, &search)
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    fn matches_source(&self, flow: &HistoryEntry) -> bool {
        let filter = self.source_filter.trim();
        if filter == ALL_SOURCES || filter.is_empty() {
            return true;
        }
        let label = if flow.listener_label.is_empty() { "default" } else { &flow.listener_label };
        label.eq_ignore_ascii_case(filter)
    }
}

fn load_detail(db_path: &PathBuf, id: i64) -> Option<HistoryDetail> {
    mitm_core::db::get_history_detail(db_path, id).ok().flatten()
}

fn status_group(code: u16) -> &'static str {
    match code {
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        500..=599 => "5xx",
        _ => "ERR",
    }
}

fn passes_tri_state(filters: &HashMap<String, TriState>, key: &str, has_includes: bool) -> bool {
    match filters.get(key).copied().unwrap_or_default() {
        TriState::Exclude => false,
        TriState::Include => true,
        TriState::Neutral => !has_includes,
    }
}

fn matches_search(flow: &HistoryEntry, search: &str) -> bool {
    if search.is_empty() {
        return true;
    }
    [&flow.url, &flow.host, &flow.path, &flow.method, &flow.content_type, &flow.listener_label]
        .iter()
        .any(|field| field.to_lowercase().contains(search))
}
