use eframe::egui::{self, RichText};
use mitm_core::credentials::{Credential, CredentialSummary, FieldKind, Vault};
use mitm_core::workspace::Workspace;
use mitm_core::utils::totp;
use crate::app::command::{Commands, QrTarget};
use crate::features::utilities::totp::{group_code, qr_buttons, show_code};
use crate::widgets::{confirm, split};
use super::lock::unlock_form;
use super::{CredentialsState, Rename, RenameTarget};
use crate::ui::icons;
use crate::widgets::button::icon_button;
use crate::widgets::list_row::list_row;
use crate::widgets::search::search_field;
use crate::widgets::section::section_with;
use mitm_core::db::Db;

pub fn show(ui: &mut egui::Ui, state: &mut CredentialsState, workspace_id: &str, workspaces: &[Workspace], db: &Db, cmds: &mut Commands) {
    state.sync(db, workspace_id);
    let now = ui.input(|i| i.time);
    let [mut list_ui, mut detail_ui] = split::side_by_side(ui, "credentials_split", &mut state.split_fraction);
    vault_list(&mut list_ui, state, workspaces, db);
    entry_detail(&mut detail_ui, state, db, now, cmds);
    delete_vault_modal(ui.ctx(), state, db);
}

enum ListAction {
    Select(String),
    AddEntry(String),
    ToggleCollapsed(String),
    Rename(RenameTarget, String),
    DeleteEntry(String),
    DeleteVault(String),
    Link { vault_id: String, workspace_id: String, linked: bool },
}

fn vault_list(ui: &mut egui::Ui, state: &mut CredentialsState, workspaces: &[Workspace], db: &Db) {
    section_with(ui, "Vaults", |ui| {
        ui.add_enabled_ui(!state.locked, |ui| {
            if icon_button(ui, icons::DOWNLOAD_SIMPLE, "Import from JSON…").clicked() {
                let vault = state.default_vault().map(|v| v.name.clone());
                state.json_import.start(vault);
            }
            if icon_button(ui, icons::PLUS, "New vault").clicked() {
                state.create_vault(db);
            }
        });
    });
    search_field(ui, &mut state.search, "Search");
    ui.add_space(4.0);

    let filter = state.search.trim().to_lowercase();
    let workspace_id = state.loaded_for.clone().unwrap_or_default();
    let (linked, other): (Vec<Vault>, Vec<Vault>) = state.vaults.iter().cloned().partition(|v| v.is_linked(&workspace_id));
    let mut action = None;
    egui::ScrollArea::vertical().id_salt("credential_list").show(ui, |ui| {
        if state.vaults.is_empty() {
            ui.weak("No vaults yet");
        }
        for vault in &linked {
            vault_rows(ui, state, db, vault, &filter, workspaces, &workspace_id, &mut action);
        }
        if !other.is_empty() {
            ui.add_space(10.0);
            ui.label(RichText::new("OTHER VAULTS").small().weak())
                .on_hover_text("Not linked to this workspace, so its requests and variables can't use them");
            for vault in &other {
                vault_rows(ui, state, db, vault, &filter, workspaces, &workspace_id, &mut action);
            }
        }
    });

    match action {
        Some(ListAction::Select(id)) => state.selected = Some(id),
        Some(ListAction::AddEntry(vault_id)) => state.create(db, &vault_id),
        Some(ListAction::ToggleCollapsed(vault_id)) => {
            if !state.collapsed.remove(&vault_id) {
                state.collapsed.insert(vault_id);
            }
        }
        Some(ListAction::Rename(target, text)) => state.rename = Some(Rename { target, text }),
        Some(ListAction::DeleteEntry(id)) => state.delete(db, &id),
        Some(ListAction::DeleteVault(id)) => state.delete_vault = Some(id),
        Some(ListAction::Link { vault_id, workspace_id, linked }) => state.set_linked(db, &vault_id, &workspace_id, linked),
        None => {}
    }
}

fn vault_rows(
    ui: &mut egui::Ui,
    state: &mut CredentialsState,
    db: &Db,
    vault: &Vault,
    filter: &str,
    workspaces: &[Workspace],
    workspace_id: &str,
    action: &mut Option<ListAction>,
) {
    let vault_matches = vault.name.to_lowercase().contains(filter);
    let entries: Vec<CredentialSummary> = state
        .summaries
        .iter()
        .filter(|s| s.vault_id == vault.id && (filter.is_empty() || vault_matches || s.name.to_lowercase().contains(filter)))
        .cloned()
        .collect();
    if !filter.is_empty() && entries.is_empty() && !vault_matches {
        return;
    }
    let collapsed = filter.is_empty() && state.collapsed.contains(&vault.id);

    let target = RenameTarget::Vault(vault.id.clone());
    let header = list_row(ui, false, |ui| {
        ui.label(RichText::new(if collapsed { icons::CARET_RIGHT } else { icons::CARET_DOWN }).weak());
        if !rename_edit(ui, state, db, &target) {
            ui.label(RichText::new(&vault.name).strong());
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let add = ui.add_enabled_ui(!state.locked, |ui| icon_button(ui, icons::PLUS, "Add credential")).inner;
            if add.clicked() {
                *action = Some(ListAction::AddEntry(vault.id.clone()));
            }
            if collapsed {
                ui.label(RichText::new(entries.len().to_string()).small().weak());
            }
        });
    });
    if header.clicked() {
        *action = Some(ListAction::ToggleCollapsed(vault.id.clone()));
    }
    header.context_menu(|ui| {
        if ui.button("Rename").clicked() {
            *action = Some(ListAction::Rename(target.clone(), vault.name.clone()));
            ui.close();
        }
        let linked_here = vault.is_linked(workspace_id);
        let toggle = if linked_here { "Unlink from this workspace" } else { "Link to this workspace" };
        if ui.button(toggle).clicked() {
            *action = Some(ListAction::Link { vault_id: vault.id.clone(), workspace_id: workspace_id.to_string(), linked: !linked_here });
            ui.close();
        }
        ui.menu_button("Workspaces", |ui| {
            for workspace in workspaces {
                let mut linked = vault.is_linked(&workspace.id);
                if ui.checkbox(&mut linked, &workspace.name).changed() {
                    *action = Some(ListAction::Link { vault_id: vault.id.clone(), workspace_id: workspace.id.clone(), linked });
                }
            }
        });
        ui.separator();
        if ui.button(RichText::new("Delete vault…").color(ui.visuals().error_fg_color)).clicked() {
            *action = Some(ListAction::DeleteVault(vault.id.clone()));
            ui.close();
        }
    });
    if collapsed {
        return;
    }

    for entry in &entries {
        let selected = state.selected.as_deref() == Some(entry.id.as_str());
        let code = state.entries.iter().find(|e| e.id == entry.id).and_then(first_code);
        let row = ui
            .indent(("credential_entry", &entry.id), |ui| {
                list_row(ui, selected, |ui| {
                    ui.label(&entry.name);
                    if let Some(code) = code {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(group_code(&code)).monospace().weak());
                        });
                    }
                })
            })
            .inner;
        if row.clicked() {
            *action = Some(ListAction::Select(entry.id.clone()));
        }
        row.context_menu(|ui| {
            if ui.button("Rename").clicked() {
                *action = Some(ListAction::Rename(RenameTarget::Entry(entry.id.clone()), entry.name.clone()));
                ui.close();
            }
            if ui.button("Delete").clicked() {
                *action = Some(ListAction::DeleteEntry(entry.id.clone()));
                ui.close();
            }
        });
    }
    if entries.is_empty() && filter.is_empty() {
        ui.indent(("empty_vault", &vault.id), |ui| ui.weak("Empty"));
    }
}

fn delete_vault_modal(ctx: &egui::Context, state: &mut CredentialsState, db: &Db) {
    let Some(id) = state.delete_vault.clone() else { return };
    let Some(vault) = state.vaults.iter().find(|v| v.id == id).cloned() else {
        state.delete_vault = None;
        return;
    };
    let count = state.summaries.iter().filter(|s| s.vault_id == id).count();
    let mut close = false;
    let modal = egui::Modal::new(egui::Id::new("delete_vault")).show(ctx, |ui| {
        ui.set_width(340.0);
        ui.heading(format!("Delete {}?", vault.name));
        ui.add_space(6.0);
        let entries = if count == 1 { "1 credential".to_string() } else { format!("{count} credentials") };
        ui.label(format!("Its {entries} are deleted too, and refs to it stop resolving in {} workspace(s).", vault.workspace_ids.len()));
        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(crate::widgets::button::danger_filled(ui, "Delete")).clicked() {
                state.remove_vault(db, &id);
                close = true;
            }
            if ui.button("Cancel").clicked() {
                close = true;
            }
        });
    });
    if close || modal.should_close() {
        state.delete_vault = None;
    }
}

fn first_code(entry: &Credential) -> Option<String> {
    let field = entry.fields.iter().find(|f| f.kind == FieldKind::Totp)?;
    let params = totp::parse_secret(&field.value).ok()?;
    Some(totp::code_at(&params, totp::now_unix()))
}

fn entry_detail(ui: &mut egui::Ui, state: &mut CredentialsState, db: &Db, now: f64, cmds: &mut Commands) {
    if state.locked {
        ui.add_space(8.0);
        unlock_form(ui, state, db);
        return;
    }
    if state.has_password {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if icon_button(ui, icons::LOCK, "Lock credentials").clicked() {
                state.lock(db);
            }
        });
    }
    let Some(id) = state.selected.clone() else { return };
    let Some(index) = state.entries.iter().position(|e| e.id == id) else { return };

    ui.horizontal(|ui| {
        let name = state.entries[index].name.clone();
        if rename_field(ui, state, db, RenameTarget::Entry(id.clone()), &name, RichText::new(&name).heading().strong()) {
            return;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let armed_id = ui.id().with(("delete_credential", &id));
            if confirm::confirm_button(ui, armed_id, RichText::new(icons::TRASH), "Delete?", "Delete credential") {
                state.delete(db, &id);
            }
        });
    });
    if let Some(error) = state.error.clone() {
        ui.horizontal(|ui| {
            ui.colored_label(ui.visuals().error_fg_color, error);
            if icon_button(ui, icons::X, "Dismiss").clicked() {
                state.error = None;
            }
        });
    }
    ui.add_space(6.0);
    if state.selected.as_deref() != Some(id.as_str()) {
        return;
    }

    let mut changed = false;
    let mut remove = None;
    egui::Grid::new(("credential_fields", &id)).num_columns(4).spacing([8.0, 6.0]).show(ui, |ui| {
        let keys: Vec<String> = state.entries[index].fields.iter().map(|f| f.key.clone()).collect();
        for (i, key) in keys.iter().enumerate() {
            rename_field(ui, state, db, RenameTarget::Field(id.clone(), key.clone()), key, RichText::new(key).strong());
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
                        if icon_button(ui, icons::COPY, "Copy").clicked() {
                            ui.ctx().copy_text(field.value.clone());
                        }
                    });
                }
                FieldKind::Secret => {
                    ui.horizontal(|ui| {
                        let eye = ui.add(egui::Button::new(icons::EYE).sense(egui::Sense::drag())).on_hover_text("Hold to reveal");
                        let revealed = eye.is_pointer_button_down_on();
                        let edit = egui::TextEdit::singleline(&mut field.value).password(!revealed).desired_width(230.0);
                        changed |= ui.add(edit).changed();
                        if icon_button(ui, icons::COPY, "Copy").clicked() {
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
                                if icon_button(ui, icons::PENCIL_SIMPLE, "Edit secret").clicked() {
                                    state.totp_draft = Some((id.clone(), key.clone(), field.value.clone()));
                                }
                            });
                        }
                        (draft, _) => {
                            let mut text = draft.unwrap_or_default();
                            let (edit, scanned) = ui
                                .horizontal(|ui| {
                                    let eye = ui.add(egui::Button::new(icons::EYE).sense(egui::Sense::drag())).on_hover_text("Hold to reveal");
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
            if icon_button(ui, icons::X, "Remove field").clicked() {
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
    if ui.button("Add field").clicked() {
        state.add_field(db, &id);
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
fn rename_field(ui: &mut egui::Ui, state: &mut CredentialsState, db: &Db, target: RenameTarget, current: &str, label: RichText) -> bool {
    if rename_edit(ui, state, db, &target) {
        return true;
    }
    let response = ui
        .add(egui::Label::new(label).sense(egui::Sense::click()))
        .on_hover_cursor(egui::CursorIcon::Text);
    if response.clicked() {
        state.rename = Some(Rename { target, text: current.to_string() });
    }
    false
}

/// The edit for a rename of `target` in progress; returns false when none is.
fn rename_edit(ui: &mut egui::Ui, state: &mut CredentialsState, db: &Db, target: &RenameTarget) -> bool {
    let Some(rename) = state.rename.as_mut().filter(|r| &r.target == target) else { return false };
    let edit = ui.add(egui::TextEdit::singleline(&mut rename.text).desired_width(140.0));
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.rename = None;
    } else if edit.lost_focus() {
        state.commit_rename(db);
    } else {
        edit.request_focus();
    }
    true
}
