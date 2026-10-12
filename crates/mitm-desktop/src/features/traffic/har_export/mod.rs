mod view;

use std::collections::{BTreeMap, HashMap, HashSet};
use mitm_core::proxy::history::HistoryEntry;
use crate::widgets::tri_state::TriState;

pub use view::show;

pub const NO_RESPONSE: &str = "No response";

/// The few fields the dialog filters on, for every stored flow.
pub struct ExportRow {
    id: i64,
    method: String,
    host: String,
    path: String,
    response: String,
    source: String,
}

impl From<HistoryEntry> for ExportRow {
    fn from(flow: HistoryEntry) -> Self {
        let response = match flow.status_code {
            0 => NO_RESPONSE.to_string(),
            code => format!("{}xx", code / 100),
        };
        let source = if flow.listener_label.is_empty() { "Default".to_string() } else { flow.listener_label };
        let path = flow.path.split('?').next().unwrap_or_default().to_string();
        Self { id: flow.id, method: flow.method.to_uppercase(), host: flow.host, path, response, source }
    }
}

#[derive(Default)]
pub struct ExportFilters {
    pub hosts: HashMap<String, TriState>,
    pub methods: HashMap<String, TriState>,
    pub responses: HashMap<String, TriState>,
    pub sources: HashMap<String, TriState>,
    pub unique_only: bool,
}

impl ExportFilters {
    /// Ids that pass every filter, newest first; duplicates keep the newest flow.
    fn select(&self, rows: &[ExportRow]) -> Vec<i64> {
        let (host, method, response, source) =
            (passes(&self.hosts), passes(&self.methods), passes(&self.responses), passes(&self.sources));
        let mut seen = HashSet::new();
        rows.iter()
            .filter(|row| host(&row.host) && method(&row.method) && response(&row.response) && source(&row.source))
            .filter(|row| !self.unique_only || seen.insert((&row.method, &row.host, &row.path)))
            .map(|row| row.id)
            .collect()
    }
}

fn passes(filters: &HashMap<String, TriState>) -> impl Fn(&str) -> bool + '_ {
    let has_includes = filters.values().any(|&s| s == TriState::Include);
    move |key| match filters.get(key).copied().unwrap_or_default() {
        TriState::Exclude => false,
        TriState::Include => true,
        TriState::Neutral => !has_includes,
    }
}

/// Values present in the history with how many flows have each, sorted by value.
#[derive(Default)]
struct Facets {
    hosts: BTreeMap<String, usize>,
    methods: BTreeMap<String, usize>,
    responses: BTreeMap<String, usize>,
    sources: BTreeMap<String, usize>,
}

impl Facets {
    fn of(rows: &[ExportRow]) -> Self {
        let mut facets = Self::default();
        for row in rows {
            *facets.hosts.entry(row.host.clone()).or_default() += 1;
            *facets.methods.entry(row.method.clone()).or_default() += 1;
            *facets.responses.entry(row.response.clone()).or_default() += 1;
            *facets.sources.entry(row.source.clone()).or_default() += 1;
        }
        facets
    }
}

#[derive(Default)]
pub struct HarExport {
    open: bool,
    // None while the flows are still loading.
    rows: Option<Vec<ExportRow>>,
    facets: Facets,
    error: Option<String>,
    host_search: String,
    // Kept between openings so a repeated export starts from the last choice.
    filters: ExportFilters,
}

impl HarExport {
    pub fn open(&mut self) {
        self.open = true;
        self.rows = None;
        self.error = None;
    }

    pub fn loaded(&mut self, result: Result<Vec<HistoryEntry>, String>) {
        if !self.open {
            return;
        }
        match result {
            Ok(flows) => {
                let rows: Vec<ExportRow> = flows.into_iter().map(ExportRow::from).collect();
                self.facets = Facets::of(&rows);
                self.rows = Some(rows);
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn selected_ids(&self) -> Vec<i64> {
        self.rows.as_deref().map(|rows| self.filters.select(rows)).unwrap_or_default()
    }

    fn matching_hosts(&self) -> impl Iterator<Item = (&String, &usize)> {
        let search = self.host_search.trim().to_lowercase();
        self.facets.hosts.iter().filter(move |(host, _)| host.to_lowercase().contains(&search))
    }

    fn set_matching_hosts(&mut self, state: TriState) {
        let hosts: Vec<String> = self.matching_hosts().map(|(host, _)| host.clone()).collect();
        for host in hosts {
            self.filters.hosts.insert(host, state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: i64, method: &str, host: &str, path: &str, status: u16) -> ExportRow {
        ExportRow::from(HistoryEntry {
            id,
            method: method.into(),
            url: format!("http://{host}{path}"),
            host: host.into(),
            path: path.into(),
            content_type: "-".into(),
            response_size: 0,
            status_code: status,
            request_headers: Vec::new(),
            response_headers: Vec::new(),
            request_body: String::new(),
            response_body: String::new(),
            phase: "response".into(),
            duration_ms: None,
            created_at: String::new(),
            is_intercepted: false,
            is_rewritten: false,
            is_failed: false,
            listener_label: String::new(),
            request_at: None,
            response_at: None,
        })
    }

    fn rows() -> Vec<ExportRow> {
        vec![
            row(6, "GET", "a.test", "/users?page=2", 200),
            row(5, "GET", "a.test", "/users?page=1", 200),
            row(4, "POST", "a.test", "/users", 201),
            row(3, "GET", "b.test", "/", 404),
            row(2, "GET", "c.test", "/", 0),
            row(1, "GET", "a.test", "/login", 302),
        ]
    }

    #[test]
    fn included_hosts_narrow_and_excluded_hosts_drop() {
        let mut filters = ExportFilters::default();
        assert_eq!(filters.select(&rows()), vec![6, 5, 4, 3, 2, 1]);

        filters.hosts.insert("b.test".into(), TriState::Exclude);
        assert_eq!(filters.select(&rows()), vec![6, 5, 4, 2, 1]);

        filters.hosts.insert("c.test".into(), TriState::Include);
        assert_eq!(filters.select(&rows()), vec![2]);
    }

    #[test]
    fn method_response_and_source_filters_combine() {
        let mut filters = ExportFilters::default();
        filters.methods.insert("GET".into(), TriState::Include);
        filters.responses.insert(NO_RESPONSE.into(), TriState::Exclude);
        assert_eq!(filters.select(&rows()), vec![6, 5, 3, 1]);

        filters.sources.insert("Default".into(), TriState::Exclude);
        assert!(filters.select(&rows()).is_empty());
    }

    #[test]
    fn unique_only_keeps_the_newest_per_method_host_and_path() {
        let filters = ExportFilters { unique_only: true, ..ExportFilters::default() };
        assert_eq!(filters.select(&rows()), vec![6, 4, 3, 2, 1]);
    }

    #[test]
    fn bulk_host_changes_follow_the_search() {
        let mut export = HarExport { facets: Facets::of(&rows()), ..HarExport::default() };
        export.host_search = "A.T".into();
        export.set_matching_hosts(TriState::Exclude);
        assert_eq!(export.filters.hosts.len(), 1);
        assert_eq!(export.filters.hosts["a.test"], TriState::Exclude);
    }
}
