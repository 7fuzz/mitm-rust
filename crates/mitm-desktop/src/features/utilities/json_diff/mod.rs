mod view;

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use similar::ChangeTag;
use crate::widgets::diff::{self, DiffLine};

pub use view::show;

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct JsonDiffPrefs {
    pub split: f32,
    pub input_split: f32,
}

impl Default for JsonDiffPrefs {
    fn default() -> Self {
        Self { split: 0.5, input_split: 0.5 }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum DiffStatus {
    Empty,
    LeftError(String),
    RightError(String),
    BothError(String, String),
    Identical,
    Diff { lines: Vec<DiffLine>, added: usize, removed: usize },
}

#[derive(Default)]
pub struct JsonDiffState {
    pub prefs: JsonDiffPrefs,
    pub left: String,
    pub right: String,
    cache: Option<(u64, Arc<DiffStatus>)>,
}

impl JsonDiffState {
    pub fn diff(&mut self) -> Arc<DiffStatus> {
        let key = self.key();
        if self.cache.as_ref().map(|(k, _)| *k) != Some(key) {
            self.cache = Some((key, Arc::new(compute(&self.left, &self.right))));
        }
        self.cache.as_ref().unwrap().1.clone()
    }

    fn key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.left.hash(&mut h);
        self.right.hash(&mut h);
        h.finish()
    }

    pub fn swap(&mut self) {
        std::mem::swap(&mut self.left, &mut self.right);
        self.cache = None;
    }

    pub fn clear(&mut self) {
        self.left.clear();
        self.right.clear();
        self.cache = None;
    }
}

fn compute(left: &str, right: &str) -> DiffStatus {
    use mitm_core::utils::json;
    if left.trim().is_empty() && right.trim().is_empty() {
        return DiffStatus::Empty;
    }
    let res_l = json::canonicalize(left);
    let res_r = json::canonicalize(right);
    match (res_l, res_r) {
        (Err(el), Err(er)) => DiffStatus::BothError(el, er),
        (Err(el), Ok(_)) => DiffStatus::LeftError(el),
        (Ok(_), Err(er)) => DiffStatus::RightError(er),
        (Ok(l), Ok(r)) => {
            if l == r {
                return DiffStatus::Identical;
            }
            let lines = diff::diff_lines(&l, &r);
            let count = |tag| lines.iter().filter(|line| line.tag == tag).count();
            let (added, removed) = (count(ChangeTag::Insert), count(ChangeTag::Delete));
            DiffStatus::Diff { lines, added, removed }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_identical_and_differences() {
        let mut state = JsonDiffState {
            left: r#"{"a": 1, "b": [2, 3]}"#.to_string(),
            right: "{\n  \"b\": [\n    2,\n    3\n  ],\n  \"a\": 1\n}".to_string(),
            ..JsonDiffState::default()
        };
        assert_eq!(*state.diff(), DiffStatus::Identical);

        state.right = r#"{"a": 2, "b": [2, 3]}"#.to_string();
        match &*state.diff() {
            DiffStatus::Diff { added, removed, .. } => assert_eq!((*added, *removed), (1, 1)),
            _ => panic!("expected Diff"),
        }

        state.right = "bad json".to_string();
        match &*state.diff() {
            DiffStatus::RightError(err) => assert!(err.contains("line 1, column")),
            _ => panic!("expected RightError"),
        }
    }
}
