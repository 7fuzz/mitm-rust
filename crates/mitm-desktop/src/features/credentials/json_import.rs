//! Imports credentials from any JSON file: leaves become rows the user ticks and assigns to entries and fields.
//! Grouping and kinds are guesses from key names; nothing here knows a particular file format.

use std::path::Path;
use eframe::egui::{self, RichText};
use mitm_core::credentials::{self, Field, FieldKind};
use serde_json::Value;
use crate::app::command::{AppCommand, Commands};
use super::CredentialsState;

/// Key endings that name a credential field: `adminPassword` is entry `admin`, field `password`.
const FIELD_SUFFIXES: [&str; 14] = [
    "TotpSecret", "ClientSecret", "ClientId", "ApiKey", "Password", "Passwd", "Username", "UserName", "Email",
    "Secret", "Token", "Totp", "Pin", "Otp",
];

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub path: String,
    pub value: String,
    pub entry: String,
    pub field: String,
    pub kind: FieldKind,
    pub picked: bool,
}

#[derive(Default)]
pub struct JsonImport {
    pub open: bool,
    json: String,
    rows: Vec<Row>,
    error: Option<String>,
}

impl JsonImport {
    pub fn start(&mut self) {
        *self = Self { open: true, ..Self::default() };
    }

    pub fn failed(&mut self, error: String) {
        *self = Self { open: true, error: Some(error), ..Self::default() };
    }

    pub fn open_with(&mut self, json: String) {
        *self = Self { open: true, json, ..Self::default() };
        self.parse();
    }

    fn parse(&mut self) {
        self.error = None;
        self.rows = Vec::new();
        if self.json.trim().is_empty() {
            return;
        }
        match serde_json::from_str::<Value>(&self.json) {
            Ok(value) => self.rows = rows(&value),
            Err(e) => self.error = Some(format!("Invalid JSON: {e}")),
        }
    }
}

/// Characters a ref can't hold become `_`.
pub fn sanitize(name: &str, fallback: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .take(64)
        .collect();
    let cleaned = cleaned.trim_matches('_');
    if cleaned.is_empty() { fallback.to_string() } else { cleaned.to_string() }
}

pub fn guess_kind(key: &str) -> FieldKind {
    let key = key.to_ascii_lowercase();
    if key.contains("expire") {
        FieldKind::Text
    } else if key.contains("totp") {
        FieldKind::Totp
    } else if ["password", "passwd", "secret", "token", "apikey", "api_key", "key", "pin", "otp"].iter().any(|w| key.contains(w)) {
        FieldKind::Secret
    } else {
        FieldKind::Text
    }
}

fn looks_like_login(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    ["email", "user", "login", "client_id", "clientid"].iter().any(|w| key.contains(w))
}

/// `adminPassword` → `("admin", "password")`; `None` without a known ending or a prefix before it.
fn split_suffix(key: &str) -> Option<(String, String)> {
    let mut capitalized = key.to_string();
    capitalized.get_mut(..1)?.make_ascii_uppercase();
    if FIELD_SUFFIXES.contains(&capitalized.as_str()) {
        return None;
    }
    FIELD_SUFFIXES.iter().find_map(|suffix| {
        let prefix = key.strip_suffix(suffix)?;
        let prefix = prefix.trim_end_matches('_');
        if prefix.is_empty() {
            return None;
        }
        let mut field = suffix.to_string();
        field[..1].make_ascii_lowercase();
        Some((prefix.to_string(), field))
    })
}

fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Every scalar leaf, grouped by the object holding it.
pub fn rows(root: &Value) -> Vec<Row> {
    let mut out = Vec::new();
    walk(root, &mut Vec::new(), &mut out);
    out
}

fn walk(value: &Value, path: &mut Vec<String>, out: &mut Vec<Row>) {
    match value {
        Value::Object(map) => {
            let owner_name = map.get("name").and_then(Value::as_str).filter(|_| path.last().is_some_and(|s| is_index(s)));
            for (key, child) in map {
                path.push(key.clone());
                match child {
                    Value::Array(items) if items.iter().all(|i| scalar(i).is_some()) => {
                        for (index, item) in items.iter().enumerate() {
                            let field = format!("{key}_{index}");
                            out.push(row(&path_with(path, &index.to_string()), item, owner(path), owner_name, &field));
                        }
                    }
                    Value::Object(_) | Value::Array(_) => walk(child, path, out),
                    leaf => {
                        if scalar(leaf).is_some() {
                            out.push(row(&path.join("."), leaf, owner(path), owner_name, key));
                        }
                    }
                }
                path.pop();
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                path.push(index.to_string());
                walk(item, path, out);
                path.pop();
            }
        }
        _ => {}
    }
}

fn is_index(segment: &str) -> bool {
    segment.bytes().all(|b| b.is_ascii_digit())
}

fn path_with(path: &[String], last: &str) -> String {
    let mut joined = path.join(".");
    joined.push('.');
    joined.push_str(last);
    joined
}

/// The segments naming the object that holds the current key: `oauthClients.0` → `oauthClients_0`.
fn owner(path: &[String]) -> String {
    let segments = &path[..path.len() - 1];
    match segments {
        [] => "imported".to_string(),
        [.., parent, index] if is_index(index) => format!("{parent}_{index}"),
        [.., parent] => parent.clone(),
    }
}

fn row(path: &str, value: &Value, owner: String, owner_name: Option<&str>, key: &str) -> Row {
    let (entry, field) = match (owner_name, split_suffix(key)) {
        (Some(name), _) => (name.to_string(), key.to_string()),
        (None, Some((prefix, field))) => (prefix, field),
        (None, None) => (owner, key.to_string()),
    };
    let kind = guess_kind(&field);
    Row {
        path: path.to_string(),
        value: scalar(value).unwrap_or_default(),
        entry: sanitize(&entry, "imported"),
        field: sanitize(&field, "value"),
        kind,
        picked: kind != FieldKind::Text || looks_like_login(&field),
    }
}

/// Writes the picked rows, creating entries as needed; a field that exists gets the new value.
/// Returns (fields written, entries touched).
pub fn apply(db_path: &Path, workspace_id: &str, rows: &[Row]) -> Result<(usize, usize), String> {
    let mut entries: Vec<(String, Vec<&Row>)> = Vec::new();
    for row in rows.iter().filter(|r| r.picked) {
        credentials::validate_name(&row.entry)?;
        credentials::validate_name(&row.field)?;
        match entries.iter_mut().find(|(name, _)| name.eq_ignore_ascii_case(&row.entry)) {
            Some((_, list)) => list.push(row),
            None => entries.push((row.entry.clone(), vec![row])),
        }
    }
    let mut fields = 0;
    for (name, picked) in &entries {
        let mut credential = match credentials::find_credential(db_path, workspace_id, name)? {
            Some(existing) => existing,
            None => credentials::create_credential(db_path, workspace_id, name)?,
        };
        for row in picked {
            match credential.field_mut(&row.field) {
                Some(field) => field.value = row.value.clone(),
                None => credential.fields.push(Field { key: row.field.clone(), kind: row.kind, extracted: false, value: row.value.clone() }),
            }
            fields += 1;
        }
        credentials::save_credential(db_path, &credential)?;
    }
    Ok((fields, entries.len()))
}

pub fn show(ctx: &egui::Context, state: &mut CredentialsState, db_path: &Path, cmds: &mut Commands) {
    if !state.json_import.open {
        return;
    }
    let mut close = false;
    let mut import = false;
    let modal = egui::Modal::new(egui::Id::new("credentials_json_import")).show(ctx, |ui| {
        ui.set_width(760.0);
        ui.heading("Import credentials from JSON");
        ui.add_space(6.0);
        let form = &mut state.json_import;
        if form.rows.is_empty() {
            if ui.button("Choose file…").clicked() {
                cmds.push(AppCommand::PickCredentialsJson);
            }
            let paste = ui.add(
                egui::TextEdit::multiline(&mut form.json)
                    .hint_text("Paste JSON")
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(8)
                    .desired_width(f32::INFINITY),
            );
            if paste.changed() {
                form.parse();
            }
        } else {
            egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                egui::Grid::new("credentials_json_rows").num_columns(5).striped(true).spacing([8.0, 4.0]).show(ui, |ui| {
                    ui.label("");
                    ui.strong("Path");
                    ui.strong("Entry");
                    ui.strong("Field");
                    ui.strong("Kind");
                    ui.end_row();
                    for (i, row) in form.rows.iter_mut().enumerate() {
                        ui.checkbox(&mut row.picked, "");
                        let shown = if row.kind == FieldKind::Text { row.value.as_str() } else { "••••" };
                        ui.add(egui::Label::new(RichText::new(&row.path).monospace()).truncate())
                            .on_hover_text(shown);
                        ui.add(egui::TextEdit::singleline(&mut row.entry).desired_width(140.0));
                        ui.add(egui::TextEdit::singleline(&mut row.field).desired_width(140.0));
                        egui::ComboBox::from_id_salt(("json_import_kind", i))
                            .selected_text(row.kind.label())
                            .width(70.0)
                            .show_ui(ui, |ui| {
                                for kind in FieldKind::ALL {
                                    ui.selectable_value(&mut row.kind, kind, kind.label());
                                }
                            });
                        ui.end_row();
                    }
                });
            });
        }
        if let Some(error) = &form.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let picked = form.rows.iter().filter(|r| r.picked).count();
            if ui.add_enabled(picked > 0, egui::Button::new(format!("Import {picked}"))).clicked() {
                import = true;
            }
            if ui.button("Cancel").clicked() {
                close = true;
            }
        });
    });
    if import {
        let workspace_id = state.loaded_for.clone().unwrap_or_default();
        match apply(db_path, &workspace_id, &state.json_import.rows) {
            Ok(_) => {
                close = true;
                state.stale = true;
            }
            Err(e) => state.json_import.error = Some(e),
        }
    }
    if close || modal.should_close() {
        state.json_import = JsonImport::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn find<'a>(rows: &'a [Row], path: &str) -> &'a Row {
        rows.iter().find(|r| r.path == path).unwrap_or_else(|| panic!("{path}"))
    }

    #[test]
    fn flat_maps_group_by_key_prefix() {
        let rows = rows(&json!({
            "credentials": { "adminEmail": "a@x", "adminPassword": "p", "adminTotpSecret": "S", "clientId": "c", "clientSecret": "s" },
            "tokens": { "dashboardToken": "t", "dashboardTokenExpiresAt": "2026" },
            "baseUrls": { "auth": "https://x" }
        }));
        let email = find(&rows, "credentials.adminEmail");
        assert_eq!((email.entry.as_str(), email.field.as_str(), email.kind, email.picked), ("admin", "email", FieldKind::Text, true));
        let totp = find(&rows, "credentials.adminTotpSecret");
        assert_eq!((totp.entry.as_str(), totp.field.as_str(), totp.kind), ("admin", "totpSecret", FieldKind::Totp));
        assert_eq!(find(&rows, "credentials.adminPassword").kind, FieldKind::Secret);
        let client = find(&rows, "credentials.clientId");
        assert_eq!((client.entry.as_str(), client.field.as_str()), ("credentials", "clientId"));
        let secret = find(&rows, "credentials.clientSecret");
        assert_eq!((secret.entry.as_str(), secret.field.as_str(), secret.kind), ("credentials", "clientSecret", FieldKind::Secret));
        assert_eq!(find(&rows, "tokens.dashboardToken").kind, FieldKind::Secret);
        assert!(!find(&rows, "tokens.dashboardTokenExpiresAt").picked);
        let url = find(&rows, "baseUrls.auth");
        assert_eq!((url.entry.as_str(), url.picked), ("baseUrls", false));
    }

    #[test]
    fn named_objects_in_lists_become_entries() {
        let rows = rows(&json!({
            "oauthClients": [{ "name": "Gateway Client", "client_id": "id", "client_secret": "s", "is_active": true }],
            "tenants": [{ "api_key": "k" }],
            "admin": { "recoveryCodes": ["a", "b"] }
        }));
        let secret = find(&rows, "oauthClients.0.client_secret");
        assert_eq!((secret.entry.as_str(), secret.field.as_str(), secret.kind, secret.picked), ("Gateway_Client", "client_secret", FieldKind::Secret, true));
        assert!(find(&rows, "oauthClients.0.client_id").picked);
        assert!(!find(&rows, "oauthClients.0.is_active").picked);
        assert_eq!(find(&rows, "tenants.0.api_key").entry, "tenants_0");
        let code = find(&rows, "admin.recoveryCodes.1");
        assert_eq!((code.entry.as_str(), code.field.as_str()), ("admin", "recoveryCodes_1"));
    }

    #[test]
    fn names_are_made_ref_safe() {
        assert_eq!(sanitize(" dg admin.v2 ", "x"), "dg_admin_v2");
        assert_eq!(sanitize("..", "x"), "x");
    }

    #[test]
    fn apply_creates_entries_and_updates_existing_fields() {
        let db = std::env::temp_dir().join(format!("mitm-json-import-{}.db", uuid::Uuid::new_v4()));
        mitm_core::db::init_database(&db, None).unwrap();
        let ws = mitm_core::workspace::create_workspace_db(&db, "W".into(), None).unwrap().id;
        let mut picked = rows(&json!({ "credentials": { "adminEmail": "a@x", "adminPassword": "p1", "note": "n" } }));
        assert_eq!(apply(&db, &ws, &picked).unwrap(), (2, 1));
        picked.iter_mut().find(|r| r.field == "password").unwrap().value = "p2".into();
        apply(&db, &ws, &picked).unwrap();
        let admin = credentials::find_credential(&db, &ws, "admin").unwrap().unwrap();
        assert_eq!(admin.fields.len(), 2);
        assert_eq!(admin.field("password").unwrap().value, "p2");
        assert!(credentials::find_credential(&db, &ws, "credentials").unwrap().is_none());
        let _ = std::fs::remove_file(&db);
    }
}
