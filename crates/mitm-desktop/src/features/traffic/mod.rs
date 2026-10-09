mod columns;
mod table;
mod toolbar;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use mitm_core::state::{HistoryDetail, HistoryEntry, HistorySettings};
use crate::features::inspector::InspectorState;
use crate::widgets::tri_state::TriState;

pub use columns::{ColumnId, ColumnLayout};

const PAGE_SIZE: usize = 200;

pub const METHODS: [&str; 6] = ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"];
pub const STATUS_GROUPS: [&str; 4] = ["2xx", "3xx", "4xx", "5xx"];
pub const FLAGS: [(&str, &str); 4] = [
    ("waiting", "Waiting"),
    ("intercepted", "Intercepted"),
    ("rewritten", "Rewritten"),
    ("failed", "Failed"),
];

#[derive(Debug, Clone, Copy, Default)]
pub struct FlowFlags {
    pub pending_response: bool,
    pub pending_intercept: bool,
    pub failed: bool,
    pub intercepted: bool,
    pub rewritten: bool,
}

impl FlowFlags {
    pub fn of(flow: &HistoryEntry) -> Self {
        let pending_response = flow.phase == "request";
        let pending_intercept = matches!(flow.phase.as_str(), "intercepted_request" | "intercepted_response");
        Self {
            pending_response,
            pending_intercept,
            failed: flow.is_failed && !pending_response && !pending_intercept,
            intercepted: flow.is_intercepted && !pending_intercept,
            rewritten: flow.is_rewritten,
        }
    }

    pub fn is_pending(&self) -> bool {
        self.pending_response || self.pending_intercept
    }

    fn matches(&self, flag: &str) -> bool {
        match flag {
            "waiting" => self.pending_response,
            "intercepted" => self.intercepted || self.pending_intercept,
            "rewritten" => self.rewritten,
            "failed" => self.failed,
            _ => false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TrafficPrefs {
    pub columns: ColumnLayout,
    pub widths: HashMap<ColumnId, f32>,
    pub search_query: String,
    pub search_headers: bool,
    pub filters_open: bool,
    pub source_filter: Option<String>,
    pub method_filters: HashMap<String, TriState>,
    pub status_filters: HashMap<String, TriState>,
    pub flag_filters: HashMap<String, TriState>,
}

impl Default for TrafficPrefs {
    fn default() -> Self {
        Self {
            columns: ColumnLayout::default(),
            widths: HashMap::new(),
            search_query: String::new(),
            search_headers: false,
            filters_open: false,
            source_filter: None,
            method_filters: neutral_filters(METHODS),
            status_filters: neutral_filters(STATUS_GROUPS),
            flag_filters: neutral_filters(FLAGS.map(|(key, _)| key)),
        }
    }
}

pub struct TrafficState {
    pub flows: VecDeque<HistoryEntry>,
    pub max_flows: usize,
    pub selected_id: Option<i64>,
    pub selected_detail: Option<HistoryDetail>,
    pub prefs: TrafficPrefs,
    pub history_limit: HistorySettings,
    pub limit_draft: HistorySettings,
    pub has_older: bool,
    /// Rows in the history table, which can be more than the flows loaded here.
    pub stored_count: u64,
    /// Ids whose stored headers match the search; summaries are loaded without headers.
    header_matches: HashSet<i64>,
    header_search_key: Option<String>,
    // Bumped to make egui forget its stored column widths and start again from `prefs.widths`.
    width_epoch: u32,
    // Top visible flow id and vertical offset from the last frame.
    scroll_anchor: Option<(i64, f32)>,
}

impl Default for TrafficState {
    fn default() -> Self {
        Self {
            flows: VecDeque::with_capacity(1000),
            max_flows: 10_000,
            selected_id: None,
            selected_detail: None,
            prefs: TrafficPrefs::default(),
            history_limit: HistorySettings::default(),
            limit_draft: HistorySettings::default(),
            has_older: false,
            stored_count: 0,
            header_matches: HashSet::new(),
            header_search_key: None,
            width_epoch: 0,
            scroll_anchor: None,
        }
    }
}

impl TrafficState {
    pub fn load_from_db(&mut self, db_path: &PathBuf) {
        self.stored_count = mitm_core::db::get_history_count(db_path, None, false).unwrap_or(0);
        self.flows.clear();
        self.has_older = true;
        self.load_older(db_path);
    }

    pub fn load_older(&mut self, db_path: &PathBuf) {
        let room = self.max_flows.saturating_sub(self.flows.len()).min(PAGE_SIZE);
        if !self.has_older || room == 0 {
            return;
        }
        let before = self.flows.back().map(|f| f.id);
        let page = mitm_core::db::get_history_page(db_path, None, false, before, room as u32).unwrap_or_default();
        self.has_older = page.len() == room;
        self.flows.extend(page);
    }

    /// Re-runs the header search in the database when the query or the toggle changed.
    pub fn sync_header_search(&mut self, db_path: &PathBuf) {
        let query = self.prefs.search_query.trim();
        let key = (self.prefs.search_headers && !query.is_empty()).then(|| query.to_string());
        if key == self.header_search_key {
            return;
        }
        self.header_matches = match &key {
            Some(query) => mitm_core::db::get_history_page(db_path, Some(query), true, None, self.max_flows as u32)
                .unwrap_or_default()
                .into_iter()
                .map(|f| f.id)
                .collect(),
            None => HashSet::new(),
        };
        self.header_search_key = key;
    }

    /// Default order, every column shown, default widths.
    pub fn reset_columns(&mut self) {
        self.prefs.columns = ColumnLayout::default();
        self.prefs.widths.clear();
        self.width_epoch += 1;
    }

    pub fn set_history_limit(&mut self, settings: HistorySettings) {
        self.limit_draft = settings.clone();
        self.history_limit = settings;
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

        // An update for an old flow that isn't loaded also lands here; only newer ids are new rows.
        if self.flows.front().is_none_or(|newest| entry.id > newest.id) {
            self.stored_count += 1;
            if self.history_limit.limiter_enabled {
                self.stored_count = self.stored_count.min(u64::from(self.history_limit.max_rows));
            }
        }
        if self.flows.len() >= self.max_flows {
            self.flows.pop_back();
            self.has_older = true;
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

    pub fn remove(&mut self, id: i64) {
        self.flows.retain(|f| f.id != id);
        self.stored_count = self.stored_count.saturating_sub(1);
        if self.selected_id == Some(id) {
            self.selected_id = None;
            self.selected_detail = None;
        }
    }

    pub fn clear(&mut self) {
        self.flows.clear();
        self.stored_count = 0;
        self.has_older = false;
        self.header_search_key = None;
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

    pub fn active_filter_count(&self) -> usize {
        let count = |filters: &HashMap<String, TriState>| filters.values().filter(|&&v| v != TriState::Neutral).count();
        count(&self.prefs.method_filters)
            + count(&self.prefs.status_filters)
            + count(&self.prefs.flag_filters)
            + usize::from(self.prefs.source_filter.is_some())
    }

    pub fn reset_filters(&mut self) {
        self.prefs.source_filter = None;
        for filters in [&mut self.prefs.method_filters, &mut self.prefs.status_filters, &mut self.prefs.flag_filters] {
            filters.values_mut().for_each(|v| *v = TriState::Neutral);
        }
    }

    pub fn filtered_indices(&self) -> Vec<usize> {
        let search = self.prefs.search_query.trim().to_lowercase();
        self.flows
            .iter()
            .enumerate()
            .filter(|(_, flow)| {
                self.matches_source(flow)
                    && passes_tri_state(&self.prefs.method_filters, Some(&flow.method.to_uppercase()))
                    && passes_tri_state(&self.prefs.status_filters, status_group(flow.status_code))
                    && self.passes_flags(FlowFlags::of(flow))
                    && (matches_search(flow, &search) || self.matches_headers(flow, &search))
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    fn matches_headers(&self, flow: &HistoryEntry, search: &str) -> bool {
        if !self.prefs.search_headers || search.is_empty() {
            return false;
        }
        self.header_matches.contains(&flow.id)
            || flow
                .request_headers
                .iter()
                .chain(&flow.response_headers)
                .any(|(k, v)| k.to_lowercase().contains(search) || v.to_lowercase().contains(search))
    }

    fn matches_source(&self, flow: &HistoryEntry) -> bool {
        let Some(filter) = &self.prefs.source_filter else {
            return true;
        };
        let label = if flow.listener_label.is_empty() { "Default" } else { &flow.listener_label };
        label.eq_ignore_ascii_case(filter)
    }

    fn passes_flags(&self, flags: FlowFlags) -> bool {
        let state_of = |flag: &str| self.prefs.flag_filters.get(flag).copied().unwrap_or_default();
        let mut has_include = false;
        let mut matches_include = false;
        for (flag, _) in FLAGS {
            match state_of(flag) {
                TriState::Exclude if flags.matches(flag) => return false,
                TriState::Include => {
                    has_include = true;
                    matches_include |= flags.matches(flag);
                }
                _ => {}
            }
        }
        !has_include || matches_include
    }
}

pub fn flow_status_badge(ui: &mut eframe::egui::Ui, code: u16, flags: &FlowFlags) {
    use crate::ui::theme::Tone;
    use crate::widgets::pills::{badge, render_status_pill};
    if flags.pending_intercept {
        badge(ui, "PAUSED", Tone::Warn);
    } else if flags.pending_response {
        badge(ui, "WAITING", Tone::Danger);
    } else if code == 0 {
        badge(ui, "FAIL", Tone::Danger);
    } else if flags.failed {
        badge(ui, &format!("{} (ERR)", code), Tone::Danger);
    } else {
        render_status_pill(ui, code);
    }
}

fn neutral_filters<const N: usize>(keys: [&str; N]) -> HashMap<String, TriState> {
    keys.iter().map(|k| (k.to_string(), TriState::Neutral)).collect()
}

fn load_detail(db_path: &PathBuf, id: i64) -> Option<HistoryDetail> {
    mitm_core::db::get_history_detail(db_path, id).ok().flatten()
}

fn status_group(code: u16) -> Option<&'static str> {
    match code {
        200..=299 => Some("2xx"),
        300..=399 => Some("3xx"),
        400..=499 => Some("4xx"),
        500.. => Some("5xx"),
        _ => None,
    }
}

fn passes_tri_state(filters: &HashMap<String, TriState>, key: Option<&str>) -> bool {
    let has_includes = filters.values().any(|&s| s == TriState::Include);
    match key.and_then(|k| filters.get(k)).copied().unwrap_or_default() {
        TriState::Exclude => false,
        TriState::Include => true,
        TriState::Neutral => !has_includes,
    }
}

fn matches_search(flow: &HistoryEntry, search: &str) -> bool {
    if search.is_empty() {
        return true;
    }
    flow.status_code.to_string().contains(search)
        || [&flow.url, &flow.host, &flow.path, &flow.method, &flow.content_type, &flow.listener_label]
            .iter()
            .any(|field| field.to_lowercase().contains(search))
}

pub fn show(
    ui: &mut eframe::egui::Ui,
    state: &mut TrafficState,
    inspector: &mut InspectorState,
    listeners: &[mitm_core::state::ListenerConfig],
    folders: &[mitm_core::collections::CollectionTreeItem],
    cmds: &mut crate::app::command::Commands,
) {
    let visible = state.filtered_indices();
    toolbar::show(ui, state, listeners, visible.len(), cmds);
    ui.separator();

    if !inspector.prefs.open || inspector.prefs.popped_out || state.selected_flow().is_none() {
        table::show(ui, state, &visible, folders, cmds);
        return;
    }
    let [mut table_ui, mut inspector_ui] =
        crate::widgets::split::stacked(ui, "traffic_inspector_split", &mut inspector.prefs.table_fraction);
    table::show(&mut table_ui, state, &visible, folders, cmds);
    crate::features::inspector::show(&mut inspector_ui, inspector, state, cmds);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flow(id: i64) -> HistoryEntry {
        HistoryEntry {
            id,
            method: "GET".into(),
            url: format!("http://example.test/{id}"),
            host: "example.test".into(),
            path: format!("/{id}"),
            content_type: "-".into(),
            response_size: 0,
            status_code: 0,
            request_headers: Vec::new(),
            response_headers: Vec::new(),
            request_body: String::new(),
            response_body: String::new(),
            phase: "request".into(),
            duration_ms: None,
            created_at: String::new(),
            is_intercepted: false,
            is_rewritten: false,
            is_failed: false,
            listener_label: String::new(),
            request_at: None,
            response_at: None,
        }
    }

    #[test]
    fn reset_columns_restores_order_visibility_and_widths() {
        let mut state = TrafficState::default();
        state.prefs.columns.move_next_to(ColumnId::Path, ColumnId::Id, false);
        state.prefs.columns.set_shown(ColumnId::Source, false);
        state.prefs.widths.insert(ColumnId::Host, 512.0);
        let epoch = state.width_epoch;

        state.reset_columns();
        assert_eq!(state.prefs.columns, ColumnLayout::default());
        assert!(state.prefs.widths.is_empty());
        assert_ne!(state.width_epoch, epoch);
    }

    #[test]
    fn stored_count_follows_new_rows_and_the_limit() {
        let mut state = TrafficState { stored_count: 40, ..TrafficState::default() };
        state.upsert(flow(10));
        state.upsert(flow(11));
        state.upsert(flow(11));
        assert_eq!(state.stored_count, 42);

        // An update for an older flow that wasn't loaded isn't a new row.
        state.upsert(flow(3));
        assert_eq!(state.stored_count, 42);

        state.remove(11);
        assert_eq!(state.stored_count, 41);

        state.history_limit = HistorySettings { limiter_enabled: true, max_rows: 41 };
        state.upsert(flow(12));
        assert_eq!(state.stored_count, 41);

        state.clear();
        assert_eq!(state.stored_count, 0);
    }
}
