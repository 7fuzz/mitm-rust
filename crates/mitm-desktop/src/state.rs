use std::collections::VecDeque;
use std::path::PathBuf;
use mitm_core::state::{HistoryDetail, HistoryEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TriState {
    #[default]
    Neutral,
    Include,
    Exclude,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TabKind {
    Traffic,
    Inspector,
    Repeater,
    Fuzzer,
    Intercept,
    Collections,
    WebSockets,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorPhase {
    Request,
    Response,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorViewMode {
    Headers,
    Formatted,
    Raw,
    Hex,
}

#[derive(Debug, Clone)]
pub enum InterceptActionCmd {
    Forward {
        flow_id: String,
        url: Option<String>,
        method: Option<String>,
        headers: Option<Vec<(String, String)>>,
        body: Option<Vec<u8>>,
    },
    Drop { flow_id: String },
    ToggleMaster { enabled: bool, mode: String },
    SaveRules { rules: Vec<mitm_core::db::InterceptRule> },
}

#[derive(Debug, Clone)]
pub struct RepeaterTabState {
    pub id: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: String,
    pub body: String,
    pub is_loading: bool,
    pub last_result: Option<mitm_core::repeater::execute::SentRequest>,
    pub error: Option<String>,
}

impl Default for RepeaterTabState {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "New Request".to_string(),
            method: "GET".to_string(),
            url: "https://httpbin.org/get".to_string(),
            headers: "User-Agent: MITM-Native/0.2\nAccept: */*".to_string(),
            body: String::new(),
            is_loading: false,
            last_result: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FuzzResultRow {
    pub index: usize,
    pub payload: String,
    pub status_code: u16,
    pub size: u64,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
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

pub enum RepeaterActionCmd {
    Send { tab_index: usize, tab: mitm_core::repeater::RepeaterTab },
}

pub enum FuzzerActionCmd {
    Start {
        template: mitm_core::fuzzer::FuzzTemplate,
        payloads: Vec<String>,
        concurrency: usize,
    },
    Stop,
}

pub struct UiState {
    pub flows: VecDeque<HistoryEntry>,
    pub max_flows: usize,
    pub selected_flow_id: Option<i64>,
    pub selected_flow_detail: Option<HistoryDetail>,

    pub inspector_phase: InspectorPhase,
    pub inspector_mode: InspectorViewMode,
    pub header_search_query: String,

    pub intercept_enabled: bool,
    pub intercept_mode: String,
    pub pending_intercepts: Vec<mitm_core::state::PendingFlowPayload>,
    pub active_intercept_id: Option<String>,
    pub intercept_rules: Vec<mitm_core::db::InterceptRule>,
    pub intercept_action_queue: Vec<InterceptActionCmd>,

    pub repeater_tabs: Vec<RepeaterTabState>,
    pub active_repeater_index: usize,
    pub repeater_action_queue: Vec<RepeaterActionCmd>,

    pub fuzzer_state: FuzzerState,
    pub fuzzer_action_queue: Vec<FuzzerActionCmd>,

    pub workspaces: Vec<mitm_core::workspace::Workspace>,
    pub active_workspace_id: String,
    pub collection_trees: Vec<mitm_core::collections::CollectionTreeItem>,
    #[allow(dead_code)]
    pub selected_collection_request: Option<mitm_core::collections::RequestItem>,

    pub ws_connections: Vec<mitm_core::state::WebSocketConn>,
    pub selected_ws_connection_id: Option<i64>,
    pub ws_messages: Vec<mitm_core::state::WebSocketMessage>,
    #[allow(dead_code)]
    pub ws_send_payload: String,
    pub ws_filter_direction: String,

    pub edit_intercept_url: String,
    pub edit_intercept_method: String,
    pub edit_intercept_headers: String,
    pub edit_intercept_body: String,

    pub new_rule_phase: String,
    pub new_rule_field: String,
    pub new_rule_operator: String,
    pub new_rule_value: String,
    pub new_rule_action: String,

    pub search_query: String,
    pub source_filter: String,
    pub method_filters: std::collections::HashMap<String, TriState>,
    pub status_filters: std::collections::HashMap<String, TriState>,
    pub auto_scroll: bool,

    pub proxy_running: bool,
    pub proxy_error: Option<String>,
    pub listeners: Vec<mitm_core::state::ListenerConfig>,
    pub active_listeners_summary: String,
    pub new_listener_label: String,
    pub new_listener_address: String,
    pub request_open_tab: Option<TabKind>,
}

impl Default for UiState {
    fn default() -> Self {
        let mut method_filters = std::collections::HashMap::new();
        for m in &["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"] {
            method_filters.insert(m.to_string(), TriState::Neutral);
        }
        let mut status_filters = std::collections::HashMap::new();
        for s in &["2xx", "3xx", "4xx", "5xx", "ERR"] {
            status_filters.insert(s.to_string(), TriState::Neutral);
        }

        Self {
            flows: VecDeque::with_capacity(1000),
            max_flows: 10_000,
            selected_flow_id: None,
            selected_flow_detail: None,
            inspector_phase: InspectorPhase::Request,
            inspector_mode: InspectorViewMode::Headers,
            header_search_query: String::new(),
            intercept_enabled: false,
            intercept_mode: "request".to_string(),
            pending_intercepts: Vec::new(),
            active_intercept_id: None,
            intercept_rules: Vec::new(),
            intercept_action_queue: Vec::new(),
            repeater_tabs: vec![RepeaterTabState::default()],
            active_repeater_index: 0,
            repeater_action_queue: Vec::new(),
            fuzzer_state: FuzzerState::default(),
            fuzzer_action_queue: Vec::new(),
            workspaces: Vec::new(),
            active_workspace_id: String::new(),
            collection_trees: Vec::new(),
            selected_collection_request: None,
            ws_connections: Vec::new(),
            selected_ws_connection_id: None,
            ws_messages: Vec::new(),
            ws_send_payload: String::new(),
            ws_filter_direction: "ALL".to_string(),
            edit_intercept_url: String::new(),
            edit_intercept_method: "GET".to_string(),
            edit_intercept_headers: String::new(),
            edit_intercept_body: String::new(),
            new_rule_phase: "request".to_string(),
            new_rule_field: "url".to_string(),
            new_rule_operator: "contains".to_string(),
            new_rule_value: String::new(),
            new_rule_action: "intercept".to_string(),
            search_query: String::new(),
            source_filter: "ALL".to_string(),
            method_filters,
            status_filters,
            auto_scroll: true,
            proxy_running: false,
            proxy_error: None,
            listeners: Vec::new(),
            active_listeners_summary: String::new(),
            new_listener_label: String::new(),
            new_listener_address: "127.0.0.1:8082".to_string(),
            request_open_tab: None,
        }
    }
}

impl UiState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_workspaces_and_collections(&mut self, db_path: &PathBuf) {
        if let Ok(workspaces) = mitm_core::workspace::get_workspaces_db(db_path) {
            self.workspaces = workspaces;
        }
        if self.active_workspace_id.is_empty() {
            if let Some(first) = self.workspaces.first() {
                self.active_workspace_id = first.id.clone();
            } else {
                self.active_workspace_id = mitm_core::repeater::REPEATER_WORKSPACE_ID.to_string();
            }
        }
        if let Ok(tree) = mitm_core::collections::get_collections_db(db_path, &self.active_workspace_id) {
            self.collection_trees = tree;
        }
    }

    pub fn load_ws_connections(&mut self, db_path: &PathBuf) {
        if let Ok(conns) = mitm_core::db::ws_db::load_ws_connections(db_path) {
            self.ws_connections = conns;
        }
    }

    pub fn select_ws_connection(&mut self, conn_id: i64, db_path: &PathBuf) {
        self.selected_ws_connection_id = Some(conn_id);
        if let Ok(msgs) = mitm_core::db::ws_db::load_ws_messages(db_path, conn_id, None) {
            self.ws_messages = msgs;
        }
    }

    pub fn send_flow_to_repeater(&mut self, flow: &HistoryEntry, detail: Option<&HistoryDetail>) {
        let headers_str = if let Some(d) = detail {
            d.request_headers.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join("\n")
        } else {
            flow.request_headers.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join("\n")
        };
        let body_str = if let Some(d) = detail {
            d.request_body.clone()
        } else {
            flow.request_body.clone()
        };

        let name = format!("{} {}", flow.method, flow.path.chars().take(20).collect::<String>());
        let tab = RepeaterTabState {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            method: flow.method.clone(),
            url: flow.url.clone(),
            headers: headers_str,
            body: body_str,
            is_loading: false,
            last_result: None,
            error: None,
        };
        self.repeater_tabs.push(tab);
        self.active_repeater_index = self.repeater_tabs.len() - 1;
    }

    pub fn send_flow_to_fuzzer(&mut self, flow: &HistoryEntry, detail: Option<&HistoryDetail>) {
        let headers_str = if let Some(d) = detail {
            d.request_headers.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join("\n")
        } else {
            flow.request_headers.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join("\n")
        };
        let body_str = if let Some(d) = detail {
            d.request_body.clone()
        } else {
            flow.request_body.clone()
        };

        self.fuzzer_state.method = flow.method.clone();
        self.fuzzer_state.url = flow.url.clone();
        self.fuzzer_state.headers = headers_str;
        self.fuzzer_state.body = body_str;
    }

    pub fn select_flow(&mut self, flow_id: i64, db_path: &PathBuf) {
        self.selected_flow_id = Some(flow_id);
        if let Ok(detail) = mitm_core::db::get_history_detail(db_path, flow_id) {
            self.selected_flow_detail = detail;
        }
    }

    pub fn add_pending_intercept(&mut self, payload: mitm_core::state::PendingFlowPayload) {
        let is_first = self.pending_intercepts.is_empty();
        let flow_id = payload.flow_id.clone();
        self.pending_intercepts.push(payload);
        if is_first || self.active_intercept_id.is_none() {
            self.select_active_intercept(&flow_id);
        }
    }

    pub fn remove_pending_intercept(&mut self, flow_id: &str) {
        self.pending_intercepts.retain(|p| p.flow_id != flow_id);
        if self.active_intercept_id.as_deref() == Some(flow_id) {
            if let Some(next) = self.pending_intercepts.first() {
                let id = next.flow_id.clone();
                self.select_active_intercept(&id);
            } else {
                self.active_intercept_id = None;
                self.edit_intercept_url.clear();
                self.edit_intercept_body.clear();
                self.edit_intercept_headers.clear();
            }
        }
    }

    pub fn select_active_intercept(&mut self, flow_id: &str) {
        if let Some(p) = self.pending_intercepts.iter().find(|p| p.flow_id == flow_id) {
            self.active_intercept_id = Some(flow_id.to_string());
            self.edit_intercept_url = p.url.clone();
            self.edit_intercept_method = p.method.clone();
            self.edit_intercept_headers = p.headers
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join("\n");
            self.edit_intercept_body = p.body_text.clone();
        }
    }

    pub fn parse_edited_headers(&self) -> Vec<(String, String)> {
        self.edit_intercept_headers
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(2, ':');
                let key = parts.next()?.trim().to_string();
                let val = parts.next()?.trim().to_string();
                if !key.is_empty() {
                    Some((key, val))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn load_history_from_db(&mut self, db_path: &PathBuf) {
        if let Ok(entries) = mitm_core::db::get_history_logs(db_path, 500, 0, None, None, None, false) {
            self.flows.clear();
            for entry in entries {
                self.flows.push_back(entry);
            }
        }
    }

    pub fn add_or_update_flow(&mut self, entry: HistoryEntry) {
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
            if let Some(resp_at) = entry.response_at {
                existing.response_at = Some(resp_at);
            }
            return;
        }

        if self.flows.len() >= self.max_flows {
            self.flows.pop_back();
        }
        self.flows.push_front(entry);
    }

    #[allow(dead_code)]
    pub fn add_flow(&mut self, entry: HistoryEntry) {
        self.add_or_update_flow(entry);
    }

    pub fn update_flow(&mut self, id: i64, status: u16, size: u64, duration_ms: u64) {
        if let Some(flow) = self.flows.iter_mut().find(|f| f.id == id) {
            flow.status_code = status;
            flow.response_size = size;
            flow.duration_ms = Some(duration_ms);
            flow.phase = "response".to_string();
        }
    }

    pub fn refresh_selected_flow_detail(&mut self, db_path: &PathBuf) {
        if let Some(id) = self.selected_flow_id {
            if let Ok(detail) = mitm_core::db::get_history_detail(db_path, id) {
                self.selected_flow_detail = detail;
            }
        }
    }

    pub fn clear_flows(&mut self) {
        self.flows.clear();
        self.selected_flow_id = None;
    }

    pub fn selected_flow(&self) -> Option<&HistoryEntry> {
        self.selected_flow_id
            .and_then(|id| self.flows.iter().find(|f| f.id == id))
    }

    pub fn reset_filters(&mut self) {
        self.search_query.clear();
        self.source_filter = "ALL".to_string();
        for v in self.method_filters.values_mut() {
            *v = TriState::Neutral;
        }
        for v in self.status_filters.values_mut() {
            *v = TriState::Neutral;
        }
    }

    pub fn available_source_labels(&self) -> Vec<String> {
        let mut labels = std::collections::BTreeSet::new();
        for l in &self.listeners {
            if !l.label.is_empty() {
                labels.insert(l.label.clone());
            }
        }
        for f in &self.flows {
            if !f.listener_label.is_empty() {
                labels.insert(f.listener_label.clone());
            }
        }
        labels.into_iter().collect()
    }

    pub fn filtered_flow_indices(&self) -> Vec<usize> {
        let search = self.search_query.trim().to_lowercase();
        let source_filter = self.source_filter.trim();

        let has_included_methods = self.method_filters.values().any(|&s| s == TriState::Include);
        let has_included_status = self.status_filters.values().any(|&s| s == TriState::Include);

        self.flows
            .iter()
            .enumerate()
            .filter(|(_, flow)| {
                if source_filter != "ALL" && !source_filter.is_empty() {
                    let flow_label = if flow.listener_label.is_empty() { "default" } else { &flow.listener_label };
                    if !flow_label.eq_ignore_ascii_case(source_filter) {
                        return false;
                    }
                }

                let m_state = self.method_filters.get(&flow.method.to_uppercase()).copied().unwrap_or(TriState::Neutral);
                if m_state == TriState::Exclude {
                    return false;
                }
                if has_included_methods && m_state != TriState::Include {
                    return false;
                }

                let status_cat = match flow.status_code {
                    200..=299 => "2xx",
                    300..=399 => "3xx",
                    400..=499 => "4xx",
                    500..=599 => "5xx",
                    _ => "ERR",
                };
                let s_state = self.status_filters.get(status_cat).copied().unwrap_or(TriState::Neutral);
                if s_state == TriState::Exclude {
                    return false;
                }
                if has_included_status && s_state != TriState::Include {
                    return false;
                }

                if !search.is_empty() {
                    let matches_url = flow.url.to_lowercase().contains(&search);
                    let matches_host = flow.host.to_lowercase().contains(&search);
                    let matches_path = flow.path.to_lowercase().contains(&search);
                    let matches_method = flow.method.to_lowercase().contains(&search);
                    let matches_type = flow.content_type.to_lowercase().contains(&search);
                    let matches_src = flow.listener_label.to_lowercase().contains(&search);
                    if !matches_url && !matches_host && !matches_path && !matches_method && !matches_type && !matches_src {
                        return false;
                    }
                }

                true
            })
            .map(|(idx, _)| idx)
            .collect()
    }
}
