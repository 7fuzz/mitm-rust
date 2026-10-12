mod view;

use std::collections::HashSet;
use std::path::Path;
use mitm_core::credentials::{self, Credential, Field, FieldKind};

pub use view::show;

const SAVE_DELAY_SECS: f64 = 0.6;

/// An entry name or field key being renamed; renames go through core so refs to them are rewritten.
#[derive(Clone)]
struct Rename {
    id: String,
    field: Option<String>,
    text: String,
}

pub struct CredentialsState {
    loaded_for: Option<String>,
    stale: bool,
    pub entries: Vec<Credential>,
    pub selected: Option<String>,
    pub search: String,
    pub split_fraction: f32,
    rename: Option<Rename>,
    /// A `totp` field whose secret is being typed in; saved secrets are never shown again.
    totp_draft: Option<(String, String, String)>,
    error: Option<String>,
    dirty: HashSet<String>,
    last_edit: f64,
}

impl Default for CredentialsState {
    fn default() -> Self {
        Self {
            loaded_for: None,
            stale: false,
            entries: Vec::new(),
            selected: None,
            search: String::new(),
            split_fraction: 0.25,
            rename: None,
            totp_draft: None,
            error: None,
            dirty: HashSet::new(),
            last_edit: 0.0,
        }
    }
}

impl CredentialsState {
    /// Reloads when the active workspace changed or something else wrote credentials (extract, import).
    pub fn sync(&mut self, db_path: &Path, workspace_id: &str) {
        if self.stale || self.loaded_for.as_deref() != Some(workspace_id) {
            self.flush(db_path);
            self.load(db_path, workspace_id);
        }
    }

    pub fn mark_stale(&mut self) {
        self.stale = true;
    }

    fn load(&mut self, db_path: &Path, workspace_id: &str) {
        self.stale = false;
        if self.loaded_for.as_deref() != Some(workspace_id) {
            self.selected = None;
            self.rename = None;
            self.totp_draft = None;
        }
        self.loaded_for = Some(workspace_id.to_string());
        let summaries = credentials::list_credentials(db_path, workspace_id).unwrap_or_default();
        self.entries = summaries.iter().filter_map(|s| credentials::load_credential(db_path, &s.id).ok()).collect();
        if !self.selected.as_ref().is_some_and(|id| self.entries.iter().any(|e| &e.id == id)) {
            self.selected = self.entries.first().map(|e| e.id.clone());
        }
        crate::widgets::template::set_credentials(&self.entries);
    }

    fn reload(&mut self, db_path: &Path) {
        if let Some(workspace_id) = self.loaded_for.clone() {
            self.load(db_path, &workspace_id);
        }
    }

    fn mark_dirty(&mut self, id: &str, now: f64) {
        self.dirty.insert(id.to_string());
        self.last_edit = now;
        crate::widgets::template::set_credentials(&self.entries);
    }

    pub fn flush_if_idle(&mut self, db_path: &Path, now: f64) {
        if !self.dirty.is_empty() && now - self.last_edit >= SAVE_DELAY_SECS {
            self.flush(db_path);
        }
    }

    pub fn flush(&mut self, db_path: &Path) {
        for id in std::mem::take(&mut self.dirty) {
            if let Some(entry) = self.entries.iter().find(|e| e.id == id) {
                if let Err(e) = credentials::save_credential(db_path, entry) {
                    self.error = Some(e);
                }
            }
        }
    }

    fn create(&mut self, db_path: &Path) {
        let Some(workspace_id) = self.loaded_for.clone() else { return };
        let name = free_name(self.entries.iter().map(|e| e.name.as_str()), "credential");
        match credentials::create_credential(db_path, &workspace_id, &name) {
            Ok(created) => {
                self.rename = Some(Rename { id: created.id.clone(), field: None, text: created.name.clone() });
                self.selected = Some(created.id);
                self.reload(db_path);
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn delete(&mut self, db_path: &Path, id: &str) {
        self.dirty.remove(id);
        if let Err(e) = credentials::delete_credential(db_path, id) {
            self.error = Some(e);
        }
        self.reload(db_path);
    }

    fn add_field(&mut self, db_path: &Path, id: &str) {
        let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) else { return };
        let key = free_name(entry.fields.iter().map(|f| f.key.as_str()), "field");
        entry.fields.push(Field { key: key.clone(), kind: FieldKind::Text, extracted: false, value: String::new() });
        self.dirty.insert(id.to_string());
        self.flush(db_path);
        self.rename = Some(Rename { id: id.to_string(), field: Some(key.clone()), text: key });
    }

    fn commit_rename(&mut self, db_path: &Path) {
        let Some(rename) = self.rename.take() else { return };
        let text = rename.text.trim();
        self.flush(db_path);
        let result = match &rename.field {
            None => credentials::rename_credential(db_path, &rename.id, text),
            Some(old) if old != text => credentials::rename_field(db_path, &rename.id, old, text),
            Some(_) => Ok(()),
        };
        if let Err(e) = result {
            self.error = Some(e);
        }
        self.reload(db_path);
    }
}

/// `base`, or `base2`, `base3`… when taken (case-insensitive, like credential names).
fn free_name<'a>(taken: impl Iterator<Item = &'a str> + Clone, base: &str) -> String {
    let is_taken = |name: &str| taken.clone().any(|t| t.eq_ignore_ascii_case(name));
    if !is_taken(base) {
        return base.to_string();
    }
    (2..).map(|n| format!("{base}{n}")).find(|name| !is_taken(name)).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_names_skip_taken_ones_ignoring_case() {
        assert_eq!(free_name(["a"].into_iter(), "field"), "field");
        assert_eq!(free_name(["Field", "field2"].into_iter(), "field"), "field3");
    }
}
