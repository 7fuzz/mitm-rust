use std::collections::HashMap;
use mitm_core::fuzzer::FuzzRow;
use crate::widgets::tri_state::TriState;
use super::FuzzerState;

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
pub(super) struct FuzzerCache {
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

impl FuzzerState {
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

    pub fn sync_response_search(&mut self, cx: &crate::app::cx::Cx) {
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
        let core = std::sync::Arc::clone(cx.core);
        let q = query.to_string();
        self.jobs.search.spawn(cx.rt, cx.ctx, async move {
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
