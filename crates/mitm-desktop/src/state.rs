use std::collections::VecDeque;
use std::path::PathBuf;
use mitm_core::state::HistoryEntry;

pub struct UiState {
    pub flows: VecDeque<HistoryEntry>,
    pub max_flows: usize,
    pub selected_flow_id: Option<i64>,

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
