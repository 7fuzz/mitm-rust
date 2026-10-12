mod json_import;
mod lock;
mod view;

use std::collections::HashSet;
use std::path::Path;
use mitm_core::credentials::{self, Credential, CredentialSummary, Field, FieldKind};
use serde::{Deserialize, Serialize};

pub use json_import::show as show_json_import;
pub use lock::{settings_section, show_unlock};
pub use view::show;

const SAVE_DELAY_SECS: f64 = 0.6;

/// An entry name or field key being renamed; renames go through core so refs to them are rewritten.
#[derive(Clone)]
struct Rename {
    id: String,
    field: Option<String>,
    text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoLock {
    AfterIdleMinutes(u32),
    Never,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CredentialsPrefs {
    pub auto_lock: AutoLock,
}

impl Default for CredentialsPrefs {
    fn default() -> Self {
        Self { auto_lock: AutoLock::AfterIdleMinutes(15) }
    }
}

pub struct CredentialsState {
    pub prefs: CredentialsPrefs,
    loaded_for: Option<String>,
    stale: bool,
    pub entries: Vec<Credential>,
    /// Names and field keys, which stay readable while locked.
    pub summaries: Vec<CredentialSummary>,
    pub has_password: bool,
    pub locked: bool,
    unlock: lock::UnlockPrompt,
    master: lock::MasterForm,
    pub json_import: json_import::JsonImport,
    last_activity: f64,
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
            prefs: CredentialsPrefs::default(),
            loaded_for: None,
            stale: false,
            entries: Vec::new(),
            summaries: Vec::new(),
            has_password: false,
            locked: false,
            unlock: lock::UnlockPrompt::default(),
            master: lock::MasterForm::default(),
            json_import: json_import::JsonImport::default(),
            last_activity: 0.0,
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
        self.has_password = credentials::has_master_password(db_path);
        self.locked = credentials::is_locked(db_path);
        self.summaries = credentials::list_credentials(db_path, workspace_id).unwrap_or_default();
        self.entries = if self.locked {
            Vec::new()
        } else {
            self.summaries.iter().filter_map(|s| credentials::load_credential(db_path, &s.id).ok()).collect()
        };
        if !self.selected.as_ref().is_some_and(|id| self.summaries.iter().any(|e| &e.id == id)) {
            self.selected = self.summaries.first().map(|e| e.id.clone());
        }
        self.publish();
    }

    /// Hands the template widget what it may show: values when unlocked, names only when locked.
    fn publish(&self) {
        if self.locked {
            let names: Vec<Credential> = self.summaries.iter().map(locked_placeholder).collect();
            crate::widgets::template::set_credentials(&names, true);
        } else {
            crate::widgets::template::set_credentials(&self.entries, false);
        }
    }

    pub fn ask_unlock(&mut self, pending: Option<crate::app::command::AppCommand>) {
        self.unlock = lock::UnlockPrompt { open: true, pending, ..Default::default() };
    }

    pub fn lock(&mut self, db_path: &Path) {
        self.flush(db_path);
        credentials::lock(db_path);
        self.stale = true;
    }

    /// The code of the entry's first `totp` field right now.
    pub fn current_code(&self, id: &str) -> Option<String> {
        use mitm_core::utils::totp;
        let entry = self.entries.iter().find(|e| e.id == id)?;
        let field = entry.fields.iter().find(|f| f.kind == FieldKind::Totp)?;
        let params = totp::parse_secret(&field.value).ok()?;
        Some(totp::code_at(&params, totp::now_unix()))
    }

    pub fn touch(&mut self, now: f64) {
        self.last_activity = now;
    }

    /// Any input counts as activity; once idle past the auto-lock limit the key is dropped.
    pub fn tick(&mut self, ctx: &eframe::egui::Context, db_path: &Path, now: f64) {
        if ctx.input(|i| !i.events.is_empty() || i.pointer.is_moving()) {
            self.last_activity = now;
        }
        let AutoLock::AfterIdleMinutes(minutes) = self.prefs.auto_lock else { return };
        if !self.has_password || self.locked {
            return;
        }
        let left = f64::from(minutes.max(1)) * 60.0 - (now - self.last_activity);
        if left <= 0.0 {
            self.lock(db_path);
        } else {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(left));
        }
    }

    fn reload(&mut self, db_path: &Path) {
        if let Some(workspace_id) = self.loaded_for.clone() {
            self.load(db_path, &workspace_id);
        }
    }

    fn mark_dirty(&mut self, id: &str, now: f64) {
        self.dirty.insert(id.to_string());
        self.last_edit = now;
        self.publish();
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

/// Moves a variable's value into `entry.field` (created as needed) and returns the ref to leave behind.
pub fn move_value(db_path: &Path, workspace_id: &str, entry: &str, field: &str, value: String) -> Result<String, String> {
    credentials::validate_name(entry)?;
    credentials::validate_name(field)?;
    let mut credential = match credentials::find_credential(db_path, workspace_id, entry)? {
        Some(existing) => existing,
        None => credentials::create_credential(db_path, workspace_id, entry)?,
    };
    if credential.field(field).is_some() {
        return Err(format!("{} already has a field \"{field}\"", credential.name));
    }
    credential.fields.push(Field { key: field.to_string(), kind: FieldKind::Secret, extracted: false, value });
    credentials::save_credential(db_path, &credential)?;
    Ok(credentials::format_ref(&credential.name, field))
}

fn locked_placeholder(summary: &CredentialSummary) -> Credential {
    Credential {
        id: summary.id.clone(),
        workspace_id: summary.workspace_id.clone(),
        name: summary.name.clone(),
        fields: summary
            .fields
            .iter()
            .map(|f| Field { key: f.key.clone(), kind: f.kind, extracted: f.extracted, value: String::new() })
            .collect(),
        notes: String::new(),
        order_index: 0,
        created_at_ms: 0,
        updated_at_ms: 0,
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

    #[test]
    fn move_value_creates_or_extends_an_entry_and_returns_its_link() {
        let db = std::env::temp_dir().join(format!("mitm-desktop-creds-{}.db", uuid::Uuid::new_v4()));
        mitm_core::db::init_database(&db, None).unwrap();
        let ws = mitm_core::workspace::create_workspace_db(&db, "W".into(), None).unwrap().id;

        assert_eq!(move_value(&db, &ws, "dgAdmin", "password", "hunter2".into()).unwrap(), "{{$cred:dgAdmin.password}}");
        assert_eq!(move_value(&db, &ws, "DGADMIN", "pin", "1234".into()).unwrap(), "{{$cred:dgAdmin.pin}}");
        assert!(move_value(&db, &ws, "dgAdmin", "password", "again".into()).is_err());
        assert!(move_value(&db, &ws, "dg admin", "x", String::new()).is_err());

        let entry = credentials::find_credential(&db, &ws, "dgAdmin").unwrap().unwrap();
        assert_eq!(entry.field("password").map(|f| (f.kind, f.value.as_str())), Some((FieldKind::Secret, "hunter2")));
        assert_eq!(entry.fields.len(), 2);
        let _ = std::fs::remove_file(&db);
    }
}
