use std::path::Path;
use eframe::egui::{self, RichText};
use mitm_core::credentials::{Credential, FieldKind};
use mitm_core::utils::totp;
use crate::app::command::{Commands, QrTarget};
use crate::features::utilities::totp::{group_code, qr_buttons, show_code};
use crate::widgets::{confirm, split};
use super::lock::unlock_form;
use super::{CredentialsState, Rename};

pub fn show(ui: &mut egui::Ui, state: &mut CredentialsState, workspace_id: &str, db_path: &Path, cmds: &mut Commands) {
    state.sync(db_path, workspace_id);
    let now = ui.input(|i| i.time);
    let [mut list_ui, mut detail_ui] = split::side_by_side(ui, "credentials_split", &mut state.split_fraction);
    entry_list(&mut list_ui, state, db_path);
    entry_detail(&mut detail_ui, state, db_path, now, cmds);
}

fn entry_list(ui: &mut egui::Ui, state: &mut CredentialsState, db_path: &Path) {
    ui.horizontal(|ui| {
        let search = ui.add(
            egui::TextEdit::singleline(&mut state.search)
                .hint_text("Search")
                .desired_width((ui.available_width() - 64.0).max(60.0)),
        );
        crate::widgets::search::claim_focus(&search);
        if ui.add_enabled(!state.locked, egui::Button::new("➕")).on_hover_text("Add credential").clicked() {
            state.create(db_path);
        }
        if ui.add_enabled(!state.locked, egui::Button::new("⇩")).on_hover_text("Import from JSON…").clicked() {
            state.json_import.start();
        }
    });
    ui.separator();

    let filter = state.search.trim().to_lowercase();
    let mut delete = None;
    egui::ScrollArea::vertical().id_salt("credential_list").show(ui, |ui| {
        for entry in state.summaries.iter().filter(|e| filter.is_empty() || e.name.to_lowercase().contains(&filter)) {
            let selected = state.selected.as_deref() == Some(entry.id.as_str());
            let row = ui
                .horizontal(|ui| {
                    let item = ui.selectable_label(selected, &entry.name);
                    if let Some(code) = state.entries.iter().find(|e| e.id == entry.id).and_then(first_code) {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(group_code(&code)).monospace().weak());
                        });
                    }
                    item
                })
                .inner;
            if row.clicked() {
                state.selected = Some(entry.id.clone());
            }
            row.context_menu(|ui| {
                if ui.button("Rename").clicked() {
                    state.rename = Some(Rename { id: entry.id.clone(), field: None, text: entry.name.clone() });
                    ui.close();
                }
                if ui.button("Delete").clicked() {
                    delete = Some(entry.id.clone());
                    ui.close();
                }
            });
        }
    });
    if let Some(id) = delete {
        state.delete(db_path, &id);
    }
}

fn first_code(entry: &Credential) -> Option<String> {
    let field = entry.fields.iter().find(|f| f.kind == FieldKind::Totp)?;
    let params = totp::parse_secret(&field.value).ok()?;
    Some(totp::code_at(&params, totp::now_unix()))
}

fn entry_detail(ui: &mut egui::Ui, state: &mut CredentialsState, db_path: &Path, now: f64, cmds: &mut Commands) {
    if state.locked {
        ui.add_space(8.0);
        unlock_form(ui, state, db_path);
        return;
    }
    if state.has_password {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if ui.small_button("🔒 Lock").clicked() {
                state.lock(db_path);
            }
        });
    }
    let Some(id) = state.selected.clone() else { return };
    let Some(index) = state.entries.iter().position(|e| e.id == id) else { return };

    ui.horizontal(|ui| {
        let name = state.entries[index].name.clone();
        if rename_field(ui, state, db_path, &id, None, &name, RichText::new(&name).heading().strong()) {
            return;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let armed_id = ui.id().with(("delete_credential", &id));
            if confirm::confirm_button(ui, armed_id, RichText::new("🗑"), "Delete?", "Delete credential") {
                state.delete(db_path, &id);
            }
        });
    });
    if let Some(error) = state.error.clone() {
        ui.horizontal(|ui| {
            ui.colored_label(ui.visuals().error_fg_color, error);
            if ui.small_button("✖").clicked() {
                state.error = None;
            }
        });
    }
    ui.separator();
    if state.selected.as_deref() != Some(id.as_str()) {
        return;
    }

    let mut changed = false;
    let mut remove = None;
    egui::Grid::new(("credential_fields", &id)).num_columns(4).spacing([8.0, 6.0]).show(ui, |ui| {
        let keys: Vec<String> = state.entries[index].fields.iter().map(|f| f.key.clone()).collect();
        for (i, key) in keys.iter().enumerate() {
            rename_field(ui, state, db_path, &id, Some(key), key, RichText::new(key).strong());
            let Some(entry) = state.entries.iter_mut().find(|e| e.id == id) else { return };
            let field = &mut entry.fields[i];
            egui::ComboBox::from_id_salt(("credential_kind", &id, i))
                .selected_text(field.kind.label())
                .width(70.0)
                .show_ui(ui, |ui| {
                    for kind in FieldKind::ALL {
                        changed |= ui.selectable_value(&mut field.kind, kind, kind.label()).changed();
                    }
                });
            let draft = state.totp_draft.as_ref().filter(|(d_id, d_key, _)| d_id == &id && d_key == key).map(|d| d.2.clone());
            match field.kind {
                FieldKind::Text => {
                    ui.horizontal(|ui| {
                        changed |= ui.add(egui::TextEdit::singleline(&mut field.value).desired_width(260.0)).changed();
                        if ui.small_button("📋").on_hover_text("Copy").clicked() {
                            ui.ctx().copy_text(field.value.clone());
                        }
                    });
                }
                FieldKind::Secret => {
                    ui.horizontal(|ui| {
                        let eye = ui.add(egui::Button::new("👁").sense(egui::Sense::drag())).on_hover_text("Hold to reveal");
                        let revealed = eye.is_pointer_button_down_on();
                        let edit = egui::TextEdit::singleline(&mut field.value).password(!revealed).desired_width(230.0);
                        changed |= ui.add(edit).changed();
                        if ui.small_button("📋").on_hover_text("Copy").clicked() {
                            crate::util::clipboard::copy_secret(ui.ctx(), &field.value);
                        }
                    });
                }
                FieldKind::Totp => {
                    let params = totp::parse_secret(&field.value).ok();
                    match (draft, params) {
                        (None, Some(params)) => {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| show_code(ui, &params, 16.0));
                                if ui.small_button("✏").on_hover_text("Edit secret").clicked() {
                                    state.totp_draft = Some((id.clone(), key.clone(), field.value.clone()));
                                }
                            });
                        }
                        (draft, _) => {
                            let mut text = draft.unwrap_or_default();
                            let (edit, scanned) = ui
                                .horizontal(|ui| {
                                    let eye = ui.add(egui::Button::new("👁").sense(egui::Sense::drag())).on_hover_text("Hold to reveal");
                                    let edit = ui.add(
                                        egui::TextEdit::singleline(&mut text)
                                            .password(!eye.is_pointer_button_down_on())
                                            .hint_text("secret or otpauth://…")
                                            .desired_width(220.0),
                                    );
                                    let target = QrTarget::CredentialField { id: id.clone(), key: key.clone() };
                                    (edit, qr_buttons(ui, cmds, target))
                                })
                                .inner;
                            let was_scanned = matches!(scanned, Some(Ok(_)));
                            match scanned {
                                Some(Ok(scanned)) => text = scanned,
                                Some(Err(e)) => state.error = Some(e),
                                None => {}
                            }
                            let has_saved = totp::parse_secret(&field.value).is_ok();
                            let escaped = edit.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape));
                            let unchanged = text.trim().is_empty() || text.trim() == field.value;
                            if has_saved && (escaped || (edit.lost_focus() && unchanged)) {
                                state.totp_draft = None;
                            } else if (edit.lost_focus() || was_scanned) && !text.trim().is_empty() {
                                match totp::parse_secret(&text) {
                                    Ok(_) => {
                                        field.value = text.trim().to_string();
                                        changed = true;
                                        state.totp_draft = None;
                                    }
                                    Err(e) => {
                                        state.error = Some(e);
                                        state.totp_draft = Some((id.clone(), key.clone(), text));
                                    }
                                }
                            } else if edit.changed() || state.totp_draft.is_some() {
                                state.totp_draft = Some((id.clone(), key.clone(), text));
                            }
                        }
                    }
                }
            }
            if ui.small_button("✖").on_hover_text("Remove field").clicked() {
                remove = Some(i);
            }
            ui.end_row();
        }
    });

    if let Some(entry) = state.entries.iter_mut().find(|e| e.id == id) {
        if let Some(i) = remove {
            entry.fields.remove(i);
            changed = true;
        }
    }
    if ui.button("➕ field").clicked() {
        state.add_field(db_path, &id);
    }
    ui.add_space(8.0);
    if let Some(entry) = state.entries.iter_mut().find(|e| e.id == id) {
        changed |= ui
            .add(egui::TextEdit::multiline(&mut entry.notes).hint_text("Notes").desired_rows(3).desired_width(f32::INFINITY))
            .changed();
    }
    if changed {
        state.mark_dirty(&id, now);
    }
}

/// A label that turns into an edit on click. Returns true while editing.
fn rename_field(
    ui: &mut egui::Ui,
    state: &mut CredentialsState,
    db_path: &Path,
    id: &str,
    field: Option<&String>,
    current: &str,
    label: RichText,
) -> bool {
    let editing = state.rename.as_ref().is_some_and(|r| r.id == id && r.field.as_ref() == field);
    if !editing {
        let response = ui
            .add(egui::Label::new(label).sense(egui::Sense::click()))
            .on_hover_cursor(egui::CursorIcon::Text);
        if response.clicked() {
            state.rename = Some(Rename { id: id.to_string(), field: field.cloned(), text: current.to_string() });
        }
        return false;
    }
    let Some(rename) = state.rename.as_mut() else { return false };
    let edit = ui.add(egui::TextEdit::singleline(&mut rename.text).desired_width(140.0));
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.rename = None;
    } else if edit.lost_focus() {
        state.commit_rename(db_path);
    } else {
        edit.request_focus();
    }
    true
}
