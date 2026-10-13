mod commands;
mod json_import;
mod lock;
mod view;

use crate::util::save_queue::SaveQueue;
use std::collections::HashSet;
use mitm_core::credentials::{self, Credential, CredentialSummary, Field, FieldKind, Vault};
use serde::{Deserialize, Serialize};
use mitm_core::db::Db;

pub use commands::{handle, Cmd};
pub use json_import::{show as show_json_import, vault_picker};
pub use lock::{settings_section, show_unlock};
pub use view::show;


/// What a rename edits; renames go through core so refs to them are rewritten.
#[derive(Clone, PartialEq)]
enum RenameTarget {
    Vault(String),
    Entry(String),
    Field(String, String),
}

#[derive(Clone)]
struct Rename {
    target: RenameTarget,
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
    pub vaults: Vec<Vault>,
    /// Every entry in every vault, with values; empty while locked.
    pub entries: Vec<Credential>,
    /// Names and field keys, which stay readable while locked.
    pub summaries: Vec<CredentialSummary>,
    /// Entries in the vaults linked to the loaded workspace, the ones its refs reach; without values while locked.
    pub linked: Vec<Credential>,
    collapsed: HashSet<String>,
    /// Asked before a vault and its entries are deleted.
    delete_vault: Option<String>,
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
    saves: SaveQueue,
}

impl Default for CredentialsState {
    fn default() -> Self {
        Self {
            prefs: CredentialsPrefs::default(),
            loaded_for: None,
            stale: false,
            vaults: Vec::new(),
            entries: Vec::new(),
            summaries: Vec::new(),
            linked: Vec::new(),
            collapsed: HashSet::new(),
            delete_vault: None,
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
            saves: SaveQueue::default(),
        }
    }
}

impl CredentialsState {
    /// Reloads when the active workspace changed or something else wrote credentials (extract, import).
    pub fn sync(&mut self, db: &Db, workspace_id: &str) {
        if self.stale || self.loaded_for.as_deref() != Some(workspace_id) {
            self.flush(db);
            self.load(db, workspace_id);
        }
    }

    pub fn mark_stale(&mut self) {
        self.stale = true;
    }

    fn load(&mut self, db: &Db, workspace_id: &str) {
        self.stale = false;
        if self.loaded_for.as_deref() != Some(workspace_id) {
            self.selected = None;
            self.rename = None;
            self.totp_draft = None;
        }
        self.loaded_for = Some(workspace_id.to_string());
        self.has_password = credentials::has_master_password(db);
        self.locked = credentials::is_locked(db);
        self.vaults = credentials::list_vaults(db).unwrap_or_default();
        self.summaries = credentials::list_credentials(db).unwrap_or_default();
        self.entries = if self.locked {
            Vec::new()
        } else {
            self.summaries.iter().filter_map(|s| credentials::load_credential(db, &s.id).ok()).collect()
        };
        if !self.selected.as_ref().is_some_and(|id| self.summaries.iter().any(|e| &e.id == id)) {
            let first_linked = self.summaries.iter().find(|s| self.is_linked(&s.vault_id)).or(self.summaries.first());
            self.selected = first_linked.map(|e| e.id.clone());
        }
        self.publish();
    }

    fn is_linked(&self, vault_id: &str) -> bool {
        let workspace_id = self.loaded_for.as_deref().unwrap_or_default();
        self.vaults.iter().any(|v| v.id == vault_id && v.is_linked(workspace_id))
    }

    /// Refreshes `linked` and hands the template widget what it may show: values when unlocked, names only when locked.
    fn publish(&mut self) {
        self.linked = if self.locked {
            self.summaries.iter().filter(|s| self.is_linked(&s.vault_id)).map(locked_placeholder).collect()
        } else {
            self.entries.iter().filter(|e| self.is_linked(&e.vault_id)).cloned().collect()
        };
        crate::widgets::template::set_credentials(&self.linked, self.locked);
    }

    /// The vault new things go into by default: the selected entry's, else the first linked one.
    pub fn default_vault(&self) -> Option<&Vault> {
        let selected = self.selected.as_ref().and_then(|id| self.summaries.iter().find(|s| &s.id == id));
        let workspace_id = self.loaded_for.as_deref().unwrap_or_default();
        selected
            .and_then(|s| self.vaults.iter().find(|v| v.id == s.vault_id))
            .or_else(|| self.vaults.iter().find(|v| v.is_linked(workspace_id)))
    }

    pub fn ask_unlock(&mut self, pending: Option<crate::app::command::AppCommand>) {
        self.unlock = lock::UnlockPrompt { open: true, pending, ..Default::default() };
    }

    pub fn lock(&mut self, db: &Db) {
        self.flush(db);
        credentials::lock(db);
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

    /// A scanned QR code for a `totp` field: saved like a typed secret.
    pub fn set_totp_secret(&mut self, id: &str, key: &str, secret: String, now: f64) {
        let Some(field) = self.entries.iter_mut().find(|e| e.id == id).and_then(|e| e.field_mut(key)) else { return };
        match mitm_core::utils::totp::parse_secret(&secret) {
            Ok(_) => {
                field.value = secret.trim().to_string();
                self.totp_draft = None;
                self.mark_dirty(id, now);
            }
            Err(e) => self.error = Some(e),
        }
    }

    pub fn show_error(&mut self, error: String) {
        self.error = Some(error);
    }

    pub fn touch(&mut self, now: f64) {
        self.last_activity = now;
    }

    /// Any input counts as activity; once idle past the auto-lock limit the key is dropped.
    pub fn tick(&mut self, ctx: &eframe::egui::Context, db: &Db, now: f64) {
        if ctx.input(|i| !i.events.is_empty() || i.pointer.is_moving()) {
            self.last_activity = now;
        }
        let AutoLock::AfterIdleMinutes(minutes) = self.prefs.auto_lock else { return };
        if !self.has_password || self.locked {
            return;
        }
        let left = f64::from(minutes.max(1)) * 60.0 - (now - self.last_activity);
        if left <= 0.0 {
            self.lock(db);
        } else {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(left));
        }
    }

    fn reload(&mut self, db: &Db) {
        if let Some(workspace_id) = self.loaded_for.clone() {
            self.load(db, &workspace_id);
        }
    }

    fn mark_dirty(&mut self, id: &str, now: f64) {
        self.saves.edited(id, now);
        self.publish();
    }

    pub fn flush_if_idle(&mut self, db: &Db, now: f64) {
        if self.saves.is_due(now) {
            self.flush(db);
            self.saves.retry_later(now);
        }
    }

    pub fn take_save_errors(&mut self) -> Vec<String> {
        self.saves.take_errors()
    }

    pub fn flush(&mut self, db: &Db) {
        for id in self.saves.take() {
            let Some(entry) = self.entries.iter().find(|e| e.id == id) else { continue };
            match credentials::save_credential(db, entry) {
                Ok(_) => self.saves.saved(&id),
                Err(e) => self.saves.failed(id, format!("Couldn't save credential: {e}")),
            }
        }
    }

    fn create(&mut self, db: &Db, vault_id: &str) {
        let taken = self.summaries.iter().filter(|s| s.vault_id == vault_id).map(|s| s.name.as_str());
        let name = free_name(taken, "credential");
        match credentials::create_credential(db, vault_id, &name) {
            Ok(created) => {
                self.collapsed.remove(vault_id);
                self.rename = Some(Rename { target: RenameTarget::Entry(created.id.clone()), text: created.name.clone() });
                self.selected = Some(created.id);
                self.reload(db);
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    /// A new vault, linked to the loaded workspace and up for renaming.
    fn create_vault(&mut self, db: &Db) {
        let name = free_name(self.vaults.iter().map(|v| v.name.as_str()), "vault");
        let created = credentials::create_vault(db, &name).and_then(|vault| {
            if let Some(workspace_id) = &self.loaded_for {
                credentials::link_vault(db, &vault.id, workspace_id)?;
            }
            Ok(vault)
        });
        match created {
            Ok(vault) => {
                self.rename = Some(Rename { target: RenameTarget::Vault(vault.id), text: vault.name });
                self.reload(db);
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    fn set_linked(&mut self, db: &Db, vault_id: &str, workspace_id: &str, linked: bool) {
        let result = if linked {
            credentials::link_vault(db, vault_id, workspace_id)
        } else {
            credentials::unlink_vault(db, vault_id, workspace_id)
        };
        if let Err(e) = result {
            self.error = Some(e.to_string());
        }
        self.reload(db);
    }

    fn remove_vault(&mut self, db: &Db, id: &str) {
        self.flush(db);
        if let Err(e) = credentials::delete_vault(db, id) {
            self.error = Some(e.to_string());
        }
        self.reload(db);
    }

    fn delete(&mut self, db: &Db, id: &str) {
        self.saves.remove(id);
        if let Err(e) = credentials::delete_credential(db, id) {
            self.error = Some(e.to_string());
        }
        self.reload(db);
    }

    fn add_field(&mut self, db: &Db, id: &str) {
        let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) else { return };
        let key = free_name(entry.fields.iter().map(|f| f.key.as_str()), "field");
        entry.fields.push(Field { key: key.clone(), kind: FieldKind::Text, extracted: false, value: String::new() });
        self.saves.insert(id);
        self.flush(db);
        self.rename = Some(Rename { target: RenameTarget::Field(id.to_string(), key.clone()), text: key });
    }

    fn commit_rename(&mut self, db: &Db) {
        let Some(rename) = self.rename.take() else { return };
        let text = rename.text.trim();
        self.flush(db);
        let result = match &rename.target {
            RenameTarget::Vault(id) => credentials::rename_vault(db, id, text),
            RenameTarget::Entry(id) => credentials::rename_credential(db, id, text),
            RenameTarget::Field(id, old) if old != text => credentials::rename_field(db, id, old, text),
            RenameTarget::Field(..) => Ok(()),
        };
        if let Err(e) = result {
            self.error = Some(e.to_string());
        }
        self.reload(db);
    }
}

/// Moves a variable's value into `vault/entry.field` (vault and entry created as needed, the vault linked to the
/// workspace) and returns the ref to leave behind.
pub fn move_value(db: &Db, workspace_id: &str, vault: &str, entry: &str, field: &str, value: String) -> Result<String, String> {
    credentials::validate_name(vault)?;
    credentials::validate_name(entry)?;
    credentials::validate_name(field)?;
    let vault_id = credentials::link_or_create_vault(db, workspace_id, vault)?;
    let mut credential = match credentials::find_in_vault(db, &vault_id, entry)? {
        Some(existing) => existing,
        None => credentials::create_credential(db, &vault_id, entry)?,
    };
    if credential.field(field).is_some() {
        return Err(format!("{} already has a field \"{field}\"", credential.name));
    }
    credential.fields.push(Field { key: field.to_string(), kind: FieldKind::Secret, extracted: false, value });
    credentials::save_credential(db, &credential)?;
    Ok(credentials::format_ref(&credential.vault, &credential.name, field))
}

fn locked_placeholder(summary: &CredentialSummary) -> Credential {
    Credential {
        id: summary.id.clone(),
        vault_id: summary.vault_id.clone(),
        vault: summary.vault.clone(),
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

/// `base`, or `base2`, `base3`… when taken (case-insensitive, like credential and vault names).
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
    fn move_value_creates_or_extends_an_entry_in_a_linked_vault_and_returns_its_link() {
        let db = mitm_core::db::Db::new(std::env::temp_dir().join(format!("mitm-desktop-creds-{}.db", uuid::Uuid::new_v4())));
        mitm_core::db::init_database(&db, None).unwrap();
        let ws = mitm_core::workspace::create_workspace_db(&db, "W".into(), None).unwrap().id;

        assert_eq!(move_value(&db, &ws, "DG", "dgAdmin", "password", "hunter2".into()).unwrap(), "{{$cred:DG/dgAdmin.password}}");
        assert_eq!(move_value(&db, &ws, "dg", "DGADMIN", "pin", "1234".into()).unwrap(), "{{$cred:DG/dgAdmin.pin}}");
        assert!(move_value(&db, &ws, "DG", "dgAdmin", "password", "again".into()).is_err());
        assert!(move_value(&db, &ws, "DG", "dg admin", "x", String::new()).is_err());
        assert!(move_value(&db, &ws, "D G", "x", "y", String::new()).is_err());

        let entry = credentials::find_credential(&db, &ws, "DG", "dgAdmin").unwrap().unwrap();
        assert_eq!(entry.field("password").map(|f| (f.kind, f.value.as_str())), Some((FieldKind::Secret, "hunter2")));
        assert_eq!(entry.fields.len(), 2);
        let _ = std::fs::remove_file(&db);
    }
}
