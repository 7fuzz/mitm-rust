use std::collections::HashMap;

use dbviewer_core::{row_label, ForeignKey, QueryResult};

pub enum FkLabel {
    Loading,
    Found(String),
    Missing,
}

/// Foreign keys of one table plus the names of the rows they point at, looked up on hover.
#[derive(Default)]
pub struct FkState {
    pub keys: Vec<ForeignKey>,
    labels: HashMap<(String, String), FkLabel>,
}

impl FkState {
    pub fn key_for(&self, column: &str) -> Option<&ForeignKey> {
        self.keys.iter().find(|fk| fk.column == column)
    }

    pub fn label(&self, column: &str, value: &str) -> Option<&FkLabel> {
        self.labels.get(&(column.to_string(), value.to_string()))
    }

    /// True the first time a value is seen, meaning the caller should fetch its label.
    pub fn start(&mut self, column: &str, value: &str) -> bool {
        let key = (column.to_string(), value.to_string());
        if self.labels.contains_key(&key) {
            return false;
        }
        self.labels.insert(key, FkLabel::Loading);
        true
    }

    pub fn resolve(&mut self, column: &str, value: String, result: Result<QueryResult, String>) {
        let Some(fk) = self.key_for(column) else { return };
        let label = match result {
            Ok(r) => match r.rows.first() {
                Some(row) => FkLabel::Found(row_label(&r.columns, row, &fk.target_column).unwrap_or_else(|| "(no name)".to_string())),
                None => FkLabel::Missing,
            },
            Err(_) => FkLabel::Missing,
        };
        self.labels.insert((column.to_string(), value), label);
    }
}
