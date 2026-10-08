mod editor;
mod rules;

use serde::{Deserialize, Serialize};
use mitm_core::state::{RewriteRule, SourceScope};
use crate::theme::Tone;

pub use rules::show;

const SAVE_DELAY_SECS: f64 = 0.6;

pub const ACTIONS: [(&str, &str); 5] = [
    ("redirect", "Redirect"),
    ("partial_request", "Partial request"),
    ("full_request", "Full request"),
    ("partial_response", "Partial response"),
    ("full_response", "Mock response"),
];

pub fn action_label(action: &str) -> &str {
    ACTIONS.iter().find(|(key, _)| *key == action).map_or(action, |(_, label)| label)
}

pub fn action_tone(action: &str) -> Tone {
    match action {
        "redirect" => Tone::Warn,
        "full_response" => Tone::Danger,
        a if a.ends_with("_response") => Tone::Success,
        _ => Tone::Info,
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RewritePrefs {
    pub list_fraction: f32,
}

impl Default for RewritePrefs {
    fn default() -> Self {
        Self { list_fraction: 0.45 }
    }
}

#[derive(Default)]
pub struct RewriteState {
    pub prefs: RewritePrefs,
    pub enabled: bool,
    pub scope: SourceScope,
    pub rules: Vec<RewriteRule>,
    pub selected: Option<String>,
    pub search: String,
    dirty_since: Option<f64>,
}

impl RewriteState {
    pub fn mark_dirty(&mut self, now: f64) {
        self.dirty_since = Some(now);
    }

    /// The rules to save once edits have settled, or right away when `force` is set (on exit).
    pub fn take_save(&mut self, now: f64, force: bool) -> Option<Vec<RewriteRule>> {
        let since = self.dirty_since?;
        if !force && now - since < SAVE_DELAY_SECS {
            return None;
        }
        self.dirty_since = None;
        Some(self.rules.clone())
    }

    pub fn add_rule(&mut self, now: f64) {
        let rule = RewriteRule {
            id: uuid::Uuid::new_v4().to_string(),
            name: "New rule".to_string(),
            enabled: true,
            action_type: "partial_request".to_string(),
            match_field: "url".to_string(),
            match_operator: "contains".to_string(),
            match_value: String::new(),
            target_part: "url".to_string(),
            target_header: None,
            match_pattern: String::new(),
            replacement_value: String::new(),
            is_regex: false,
            mock_status_code: None,
            mock_headers_json: None,
            mock_body: None,
            order_index: self.rules.len() as i32,
            created_at_ms: chrono::Utc::now().timestamp_millis(),
        };
        self.selected = Some(rule.id.clone());
        self.rules.push(rule);
        self.mark_dirty(now);
    }

    pub fn selected_index(&self) -> Option<usize> {
        let id = self.selected.as_deref()?;
        self.rules.iter().position(|r| r.id == id)
    }

    pub fn duplicate(&mut self, index: usize, now: f64) {
        let Some(original) = self.rules.get(index) else { return };
        let copy = RewriteRule {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!("{} (copy)", original.name),
            created_at_ms: chrono::Utc::now().timestamp_millis(),
            ..original.clone()
        };
        self.selected = Some(copy.id.clone());
        self.rules.insert(index + 1, copy);
        self.mark_dirty(now);
    }

    pub fn delete(&mut self, index: usize, now: f64) {
        let removed = self.rules.remove(index);
        if self.selected.as_deref() == Some(&removed.id) {
            self.selected = None;
        }
        self.mark_dirty(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_wait_for_edits_to_settle_unless_forced() {
        let mut state = RewriteState::default();
        state.add_rule(1.0);
        assert!(state.take_save(1.2, false).is_none());
        assert_eq!(state.take_save(1.7, false).map(|r| r.len()), Some(1));
        assert!(state.take_save(9.0, false).is_none());

        state.duplicate(0, 10.0);
        assert_eq!(state.rules[1].name, "New rule (copy)");
        assert_eq!(state.take_save(10.0, true).map(|r| r.len()), Some(2));
    }
}
