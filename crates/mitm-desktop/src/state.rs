use std::collections::VecDeque;
use std::path::PathBuf;
use mitm_core::state::{HistoryDetail, HistoryEntry};

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

    // Inspector tab state
    pub inspector_phase: InspectorPhase,
    pub inspector_mode: InspectorViewMode,
    pub header_search_query: String,

    // Interception state
    pub intercept_enabled: bool,
    pub intercept_mode: String,
    pub pending_intercepts: Vec<mitm_core::state::PendingFlowPayload>,
    pub active_intercept_id: Option<String>,
    pub intercept_rules: Vec<mitm_core::db::InterceptRule>,
    pub intercept_action_queue: Vec<InterceptActionCmd>,

    // Repeater state
    pub repeater_tabs: Vec<RepeaterTabState>,
    pub active_repeater_index: usize,
    pub repeater_action_queue: Vec<RepeaterActionCmd>,

    // Fuzzer state
    pub fuzzer_state: FuzzerState,
    pub fuzzer_action_queue: Vec<FuzzerActionCmd>,

    // Workspaces & Collections state
    pub workspaces: Vec<mitm_core::workspace::Workspace>,
    pub active_workspace_id: String,
    pub collection_trees: Vec<mitm_core::collections::CollectionTreeItem>,
    pub selected_collection_request: Option<mitm_core::collections::RequestItem>,

    // WebSocket state
    pub ws_connections: Vec<mitm_core::state::WebSocketConn>,
    pub selected_ws_connection_id: Option<i64>,
    pub ws_messages: Vec<mitm_core::state::WebSocketMessage>,
    pub ws_send_payload: String,
    pub ws_filter_direction: String,

    // Active intercept editing buffers
    pub edit_intercept_url: String,
    pub edit_intercept_method: String,
    pub edit_intercept_headers: String,
    pub edit_intercept_body: String,

    // New rule builder fields
    pub new_rule_phase: String,
    pub new_rule_field: String,
    pub new_rule_operator: String,
    pub new_rule_value: String,
    pub new_rule_action: String,

    // Search and filtering
    pub search_query: String,
    pub method_filter: String,
    pub status_filter: String,
    pub auto_scroll: bool,

    // Proxy runtime state
    pub proxy_running: bool,
    pub proxy_port: u16,
    pub proxy_label: String,
    pub proxy_error: Option<String>,
}

impl Default for UiState {
    fn default() -> Self {
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
            method_filter: "ALL".to_string(),
            status_filter: "ALL".to_string(),
            auto_scroll: true,
            proxy_running: false,
            proxy_port: 8080,
            proxy_label: "default".to_string(),
            proxy_error: None,
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
            // DB returns descending (newest first). Reverse so newest is appended at end.
            for entry in entries.into_iter().rev() {
                self.add_flow(entry);
            }
        }
    }

    pub fn add_flow(&mut self, entry: HistoryEntry) {
        if self.flows.len() >= self.max_flows {
            self.flows.pop_front();
        }
        self.flows.push_back(entry);
    }

    pub fn update_flow(&mut self, id: i64, status: u16, size: u64, duration_ms: u64) {
        if let Some(flow) = self.flows.iter_mut().find(|f| f.id == id) {
            flow.status_code = status;
            flow.response_size = size;
            flow.duration_ms = Some(duration_ms);
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

    pub fn filtered_flow_indices(&self) -> Vec<usize> {
        let search = self.search_query.trim().to_lowercase();
        let method_filter = self.method_filter.to_uppercase();
        let status_filter = self.status_filter.as_str();

        self.flows
            .iter()
            .enumerate()
            .filter(|(_, flow)| {
                // Method filter
                if method_filter != "ALL" && !flow.method.eq_ignore_ascii_case(&method_filter) {
                    return false;
                }

                // Status category filter
                if status_filter != "ALL" {
                    match status_filter {
                        "2xx" => if flow.status_code < 200 || flow.status_code >= 300 { return false; },
                        "3xx" => if flow.status_code < 300 || flow.status_code >= 400 { return false; },
                        "4xx" => if flow.status_code < 400 || flow.status_code >= 500 { return false; },
                        "5xx" => if flow.status_code < 500 || flow.status_code >= 600 { return false; },
                        "ERR" => if flow.status_code != 0 && !flow.is_failed { return false; },
                        _ => {}
                    }
                }

                // Search query match across url, host, path, method
                if !search.is_empty() {
                    let matches_url = flow.url.to_lowercase().contains(&search);
                    let matches_host = flow.host.to_lowercase().contains(&search);
                    let matches_path = flow.path.to_lowercase().contains(&search);
                    let matches_method = flow.method.to_lowercase().contains(&search);
                    let matches_type = flow.content_type.to_lowercase().contains(&search);
                    if !matches_url && !matches_host && !matches_path && !matches_method && !matches_type {
                        return false;
                    }
                }

                true
            })
            .map(|(idx, _)| idx)
            .collect()
    }
}
