use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use mitm_core::credentials::Credential;
use mitm_core::workspace::{EnvironmentVariable, VariableVariant};
use crate::app::command::{AppCommand, Commands};
use crate::widgets::split;
use crate::ui::icons;
use crate::ui::theme::{self, Tone};
use crate::widgets::button::icon_button;
use crate::widgets::list_row::list_row;
use crate::widgets::pills::badge;
use crate::widgets::search::search_field;
use crate::widgets::section::section_with;
use super::{Cmd, EnvironmentsState, MoveModal, VariantModal};

pub fn show(ui: &mut egui::Ui, state: &mut EnvironmentsState, credentials: &[Credential], cmds: &mut Commands) {
    let now = ui.input(|i| i.time);
    workspace_header(ui, state, cmds);
    ui.add_space(8.0);
    let [mut list_ui, mut editor_ui] = split::side_by_side(ui, "environments_split", &mut state.split_fraction);
    environment_list(&mut list_ui, state, cmds);
    environment_editor(&mut editor_ui, state, credentials, cmds, now);
}

fn workspace_header(ui: &mut egui::Ui, state: &mut EnvironmentsState, cmds: &mut Commands) {
    let active_ws = state.active_workspace().cloned();
    let Some(ws) = active_ws else { return; };

    ui.horizontal(|ui| {
        if state.renaming_workspace {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut state.rename_workspace_name)
                    .hint_text("Workspace name"),
            );
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                state.renaming_workspace = false;
            } else if resp.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let trimmed = state.rename_workspace_name.trim().to_string();
                if !trimmed.is_empty() && trimmed != ws.name {
                    cmds.push(Cmd::RenameWorkspace {
                        id: ws.id.clone(),
                        name: trimmed,
                    });
                }
                state.renaming_workspace = false;
            }
            if state.renaming_workspace {
                resp.request_focus();
            }
        } else {
            let title = ui.add(
                egui::Label::new(RichText::new(&ws.name).heading().strong())
                    .sense(egui::Sense::click()),
            ).on_hover_cursor(egui::CursorIcon::PointingHand)
             .on_hover_text("Click to rename workspace");
            if title.clicked() {
                state.renaming_workspace = true;
                state.rename_workspace_name = ws.name.clone();
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let can_delete = state.workspaces.len() > 1;
            let del_btn = egui::Button::new("Delete workspace");
            if ui.add_enabled(can_delete, del_btn)
                .on_hover_text(if can_delete { "Delete this workspace" } else { "Cannot delete the only workspace" })
                .clicked()
            {
                state.confirm_delete_workspace = true;
            }
        });
    });

    ui.add_space(2.0);

    if state.editing_description {
        let resp = ui.add(
            egui::TextEdit::singleline(&mut state.description_text)
                .hint_text("Add a description…")
                .desired_width((ui.available_width() - 80.0).max(80.0)),
        );
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            state.editing_description = false;
        } else if resp.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let trimmed = state.description_text.trim();
            let new_desc = if trimmed.is_empty() { None } else { Some(trimmed.to_string()) };
            if new_desc != ws.description {
                cmds.push(Cmd::DescribeWorkspace {
                    id: ws.id.clone(),
                    description: new_desc,
                });
            }
            state.editing_description = false;
        }
        if state.editing_description {
            resp.request_focus();
        }
    } else {
        let has_desc = ws.description.as_ref().map(|d| !d.trim().is_empty()).unwrap_or(false);
        let desc_label = if has_desc {
            RichText::new(ws.description.as_deref().unwrap()).weak()
        } else {
            RichText::new("Add a description…").weak()
        };
        let resp = ui.add(egui::Label::new(desc_label).sense(egui::Sense::click()))
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text("Click to edit description");
        if resp.clicked() {
            state.editing_description = true;
            state.description_text = ws.description.clone().unwrap_or_default();
        }
    }

    if state.confirm_delete_workspace {
        egui::Modal::new(egui::Id::new("delete_workspace_confirm_modal")).show(ui.ctx(), |ui| {
            ui.set_width(360.0);
            ui.heading("Delete workspace");
            ui.add_space(6.0);
            ui.label(format!("Are you sure you want to delete \"{}\"?", ws.name));
            ui.weak("All collections and environments in this workspace will be deleted.");
            ui.add_space(8.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(crate::widgets::button::danger_filled(ui, "Delete")).clicked() {
                    cmds.push(AppCommand::DeleteWorkspace(ws.id.clone()));
                    state.confirm_delete_workspace = false;
                }
                if ui.button("Cancel").clicked() {
                    state.confirm_delete_workspace = false;
                }
            });
        });
    }
}

fn environment_list(ui: &mut egui::Ui, state: &mut EnvironmentsState, cmds: &mut Commands) {
    section_with(ui, "Environments", |ui| {
        if icon_button(ui, icons::PLUS, "Add environment").clicked() {
            cmds.push(Cmd::Create {
                workspace_id: state.active_workspace_id.clone(),
                name: "New Environment".to_string(),
            });
        }
    });

    egui::ScrollArea::vertical().id_salt("env_list_scroll").show(ui, |ui| {
        if state.environments.is_empty() {
            ui.weak("No environments");
            return;
        }

        let mut rename_commit = None;
        let mut rename_cancel = false;

        for env in &state.environments {
            let is_selected = state.selected_environment_id.as_deref() == Some(&env.id);
            let is_active = env.is_active;

            if state.renaming_env_id.as_deref() == Some(&env.id) {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut state.rename_text)
                        .hint_text("Environment name"),
                );
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    rename_cancel = true;
                } else if resp.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let text = state.rename_text.trim().to_string();
                    if !text.is_empty() {
                        rename_commit = Some((env.id.clone(), text));
                    } else {
                        rename_cancel = true;
                    }
                }
                if rename_commit.is_none() && !rename_cancel {
                    resp.request_focus();
                }
            } else {
                let resp = list_row(ui, is_selected, |ui| {
                    ui.label(&env.name);
                    if is_active {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("●").color(theme::color(ui, Tone::Success))).on_hover_text("Active");
                        });
                    }
                });

                if resp.clicked() {
                    state.selected_environment_id = Some(env.id.clone());
                }
                if resp.double_clicked() {
                    state.renaming_env_id = Some(env.id.clone());
                    state.rename_text = env.name.clone();
                }

                resp.context_menu(|ui| {
                    if is_active {
                        if ui.button("Deactivate").clicked() {
                            cmds.push(Cmd::Select(None));
                            ui.close();
                        }
                    } else if ui.button("Set active").clicked() {
                        cmds.push(Cmd::Select(Some(env.id.clone())));
                        ui.close();
                    }
                    if ui.button("Rename").clicked() {
                        state.renaming_env_id = Some(env.id.clone());
                        state.rename_text = env.name.clone();
                        ui.close();
                    }
                    if ui.button("Duplicate").clicked() {
                        cmds.push(Cmd::Duplicate(env.id.clone()));
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Delete").clicked() {
                        cmds.push(Cmd::Delete(env.id.clone()));
                        ui.close();
                    }
                });
            }
        }

        if let Some((id, name)) = rename_commit {
            cmds.push(Cmd::Rename { id, name });
            state.renaming_env_id = None;
            state.rename_text.clear();
        }
        if rename_cancel {
            state.renaming_env_id = None;
            state.rename_text.clear();
        }
    });
}

fn environment_editor(ui: &mut egui::Ui, state: &mut EnvironmentsState, credentials: &[Credential], cmds: &mut Commands, now: f64) {
    let Some(selected_id) = state.selected_environment_id.clone() else { return };
    let Some(env_idx) = state.environments.iter().position(|e| e.id == selected_id) else { return };

    let env = &mut state.environments[env_idx];

    ui.horizontal(|ui| {
        ui.heading(&env.name);
        if env.is_active {
            badge(ui, "Active", Tone::Success);
        }
    });
    ui.add_space(4.0);

    search_field(ui, &mut state.variable_filter, "Filter variables");
    ui.add_space(4.0);

    let filter = state.variable_filter.trim().to_lowercase();
    let mut changed = false;
    let mut delete_idx = None;
    let mut open_variant_modal = None;
    let mut delete_variant_idx = None;
    let mut open_move = None;

    let total_vars = env.variables.len();
    let matching_indices: Vec<usize> = (0..total_vars)
        .filter(|&i| {
            if filter.is_empty() {
                true
            } else {
                let v = &env.variables[i];
                v.key.to_lowercase().contains(&filter) || v.value.to_lowercase().contains(&filter)
            }
        })
        .collect();

    let show_new_row = filter.is_empty();
    let row_count = matching_indices.len() + if show_new_row { 1 } else { 0 };

    TableBuilder::new(ui)
        .striped(false)
        .resizable(true)
        .auto_shrink([false, false])
        .column(Column::exact(28.0))
        .column(Column::exact(28.0))
        .column(Column::initial(160.0).at_least(80.0).clip(true))
        .column(Column::remainder().at_least(120.0).clip(true))
        .column(Column::exact(28.0))
        .header(22.0, |mut header| {
            for title in ["", "", "Key", "Value", ""] {
                header.col(|ui| {
                    ui.label(RichText::new(title).small().weak());
                });
            }
        })
        .body(|body| {
            body.rows(26.0, row_count, |mut row| {
                let row_idx = row.index();
                if row_idx < matching_indices.len() {
                    let var_idx = matching_indices[row_idx];
                    let var = &mut env.variables[var_idx];

                    row.col(|ui| {
                        changed |= ui.checkbox(&mut var.enabled, "").changed();
                    });
                    row.col(|ui| {
                        let is_secret = var.r#type == "secret";
                        let icon = if is_secret { icons::LOCK } else { icons::LOCK_OPEN };
                        let tip = if is_secret {
                            "Secret variable (masked)\nClick to change to plain"
                        } else {
                            "Plain text variable\nClick to mask as secret"
                        };
                        let btn = if is_secret {
                            egui::Button::new(RichText::new(icon).color(ui.visuals().warn_fg_color))
                        } else {
                            egui::Button::new(RichText::new(icon).weak())
                        };
                        if ui.add(btn).on_hover_text(tip).clicked() {
                            var.r#type = if is_secret { "default".to_string() } else { "secret".to_string() };
                            changed = true;
                        }
                    });
                    row.col(|ui| {
                        changed |= ui.add(
                            egui::TextEdit::singleline(&mut var.key)
                                .hint_text("Key")
                                .desired_width(ui.available_width()),
                        ).changed();
                    });
                    row.col(|ui| {
                        ui.horizontal(|ui| {
                            let has_variants = var.variants.len() > 1;
                            if has_variants {
                                let active_idx = var.active_index.min(var.variants.len() - 1);
                                let name = &var.variants[active_idx].name;
                                let cycler = ui.small_button(format!("‹ {} ›", name))
                                    .on_hover_text("Click to cycle variant\nRight-click for options");
                                if cycler.clicked() {
                                    var.active_index = (active_idx + 1) % var.variants.len();
                                    var.value = var.variants[var.active_index].value.clone();
                                    changed = true;
                                }
                                cycler.context_menu(|ui| {
                                    for (i, v) in var.variants.iter().enumerate() {
                                        if ui.selectable_label(i == active_idx, &v.name).clicked() {
                                            var.active_index = i;
                                            var.value = v.value.clone();
                                            changed = true;
                                            ui.close();
                                        }
                                    }
                                    ui.separator();
                                    if ui.button("Add variant…").clicked() {
                                        open_variant_modal = Some((var_idx, false));
                                        ui.close();
                                    }
                                    if ui.button("Rename variant…").clicked() {
                                        open_variant_modal = Some((var_idx, true));
                                        ui.close();
                                    }
                                    if ui.button("Delete variant").clicked() {
                                        delete_variant_idx = Some((var_idx, active_idx));
                                        ui.close();
                                    }
                                });
                            }

                            let is_secret = var.r#type == "secret";
                            let current = var.current_value().to_string();
                            let link = mitm_core::credentials::link_target(&current)
                                .map(|(entry, field)| (entry.to_string(), field.to_string()));
                            let resp = if let Some((entry, field)) = &link {
                                let label = ui
                                    .add(egui::Label::new(RichText::new(format!("{} {entry} · {field}", icons::LOCK)).monospace()).sense(egui::Sense::click()))
                                    .on_hover_text(&current);
                                if icon_button(ui, icons::X, "Unlink").clicked() {
                                    set_active_value(var, String::new());
                                    changed = true;
                                }
                                label
                            } else {
                                if let Some(target) = link_menu(ui, credentials) {
                                    set_active_value(var, target);
                                    changed = true;
                                }
                                let mut revealed = false;
                                if is_secret {
                                    let eye = ui.add(egui::Button::new(icons::EYE).sense(egui::Sense::drag()))
                                        .on_hover_text("Hold to reveal secret");
                                    revealed = eye.is_pointer_button_down_on()
                                        || (eye.hovered() && ui.input(|i| i.pointer.primary_down()));
                                }

                                let val_mut = if !var.variants.is_empty() {
                                    let idx = var.active_index.min(var.variants.len() - 1);
                                    &mut var.variants[idx].value
                                } else {
                                    &mut var.value
                                };

                                let mut edit = egui::TextEdit::singleline(val_mut)
                                    .hint_text("Value")
                                    .desired_width(ui.available_width());
                                if is_secret && !revealed {
                                    edit = edit.password(true);
                                }

                                let resp = ui.add(edit);
                                if resp.changed() {
                                    var.value = val_mut.clone();
                                    changed = true;
                                }
                                resp
                            };

                            resp.context_menu(|ui| {
                                if !var.variants.is_empty() {
                                    let active_idx = var.active_index.min(var.variants.len() - 1);
                                    for (i, v) in var.variants.iter().enumerate() {
                                        if ui.selectable_label(i == active_idx, &v.name).clicked() {
                                            var.active_index = i;
                                            var.value = v.value.clone();
                                            changed = true;
                                            ui.close();
                                        }
                                    }
                                    ui.separator();
                                }
                                if is_secret && link.is_none() && !current.is_empty() && ui.button("Move to credentials…").clicked() {
                                    open_move = Some(var_idx);
                                    ui.close();
                                }
                                if ui.button("Add variant…").clicked() {
                                    open_variant_modal = Some((var_idx, false));
                                    ui.close();
                                }
                                if !var.variants.is_empty() {
                                    let active_idx = var.active_index.min(var.variants.len() - 1);
                                    if ui.button("Rename variant…").clicked() {
                                        open_variant_modal = Some((var_idx, true));
                                        ui.close();
                                    }
                                    if ui.button("Delete variant").clicked() {
                                        delete_variant_idx = Some((var_idx, active_idx));
                                        ui.close();
                                    }
                                }
                            });
                        });
                    });
                    row.col(|ui| {
                        if icon_button(ui, icons::X, "Delete variable").clicked() {
                            delete_idx = Some(var_idx);
                        }
                    });
                } else if show_new_row {
                    row.col(|ui| {
                        ui.add_enabled(false, egui::Checkbox::new(&mut true, ""));
                    });
                    row.col(|_| {});
                    let mut key_changed = false;
                    let mut val_changed = false;
                    row.col(|ui| {
                        key_changed = ui.add(
                            egui::TextEdit::singleline(&mut state.new_var_key)
                                .hint_text("Add key")
                                .desired_width(ui.available_width()),
                        ).changed();
                    });
                    row.col(|ui| {
                        val_changed = ui.add(
                            egui::TextEdit::singleline(&mut state.new_var_value)
                                .hint_text("Add value")
                                .desired_width(ui.available_width()),
                        ).changed();
                    });
                    row.col(|_| {});

                    if key_changed || val_changed {
                        let key = std::mem::take(&mut state.new_var_key);
                        let value = std::mem::take(&mut state.new_var_value);
                        env.variables.push(EnvironmentVariable {
                            key,
                            value,
                            enabled: true,
                            r#type: "default".to_string(),
                            active_index: 0,
                            variants: Vec::new(),
                        });
                        changed = true;
                    }
                }
            });
        });

    if let Some((v_idx, active_idx)) = delete_variant_idx {
        let env = &mut state.environments[env_idx];
        if v_idx < env.variables.len() {
            let var = &mut env.variables[v_idx];
            if active_idx < var.variants.len() {
                var.variants.remove(active_idx);
                if var.active_index >= var.variants.len() && !var.variants.is_empty() {
                    var.active_index = var.variants.len() - 1;
                }
                var.value = var.current_value().to_string();
                changed = true;
            }
        }
    }

    if let Some((v_idx, is_rename)) = open_variant_modal {
        let env = &state.environments[env_idx];
        if v_idx < env.variables.len() {
            let var = &env.variables[v_idx];
            let name = if is_rename {
                if !var.variants.is_empty() {
                    let idx = var.active_index.min(var.variants.len() - 1);
                    var.variants[idx].name.clone()
                } else {
                    String::new()
                }
            } else {
                format!("v{}", var.variants.len() + 1)
            };
            state.variant_modal = Some(VariantModal {
                var_idx: v_idx,
                is_rename,
                name,
            });
        }
    }

    if let Some(idx) = delete_idx {
        state.environments[env_idx].variables.remove(idx);
        changed = true;
    }

    if changed {
        state.mark_dirty(&selected_id, now);
    }

    if let Some(var_idx) = open_move {
        let key = &state.environments[env_idx].variables[var_idx].key;
        state.move_modal = Some(MoveModal {
            env_id: selected_id.clone(),
            var_idx,
            entry: credentials.first().map(|c| c.name.clone()).unwrap_or_default(),
            field: field_key_from(key),
            error: None,
        });
    }

    variant_modal(ui.ctx(), state, env_idx, now);
    move_modal(ui.ctx(), state, credentials, cmds);
}

fn set_active_value(var: &mut EnvironmentVariable, value: String) {
    if !var.variants.is_empty() {
        let idx = var.active_index.min(var.variants.len() - 1);
        var.variants[idx].value = value.clone();
    }
    var.value = value;
}

/// Picks a credential field to link the variant to; returns the ref to store as its value.
fn link_menu(ui: &mut egui::Ui, credentials: &[Credential]) -> Option<String> {
    if credentials.iter().all(|c| c.fields.is_empty()) {
        return None;
    }
    let mut picked = None;
    ui.menu_button(icons::LINK, |ui| {
        for credential in credentials.iter().filter(|c| !c.fields.is_empty()) {
            ui.menu_button(&credential.name, |ui| {
                for field in &credential.fields {
                    if ui.button(&field.key).clicked() {
                        picked = Some(mitm_core::credentials::format_ref(&credential.name, &field.key));
                        ui.close();
                    }
                }
            });
        }
    })
    .response
    .on_hover_text("Link to a credential");
    picked
}

/// A variable key as a credential field key: characters a ref can't hold become `_`.
pub(super) fn field_key_from(key: &str) -> String {
    let cleaned: String = key
        .trim()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .take(64)
        .collect();
    if cleaned.is_empty() { "value".to_string() } else { cleaned }
}

fn move_modal(ctx: &egui::Context, state: &mut EnvironmentsState, credentials: &[Credential], cmds: &mut Commands) {
    let Some(modal) = &mut state.move_modal else { return };
    let mut close = false;
    egui::Modal::new(egui::Id::new("move_to_credentials")).show(ctx, |ui| {
        ui.set_width(360.0);
        ui.heading("Move to credentials");
        ui.add_space(6.0);
        egui::Grid::new("move_to_credentials_grid").num_columns(2).show(ui, |ui| {
            ui.label("Entry");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut modal.entry).desired_width(160.0));
                if !credentials.is_empty() {
                    ui.menu_button(icons::CARET_DOWN, |ui| {
                        for credential in credentials {
                            if ui.button(&credential.name).clicked() {
                                modal.entry = credential.name.clone();
                                ui.close();
                            }
                        }
                    });
                }
            });
            ui.end_row();
            ui.label("Field");
            ui.add(egui::TextEdit::singleline(&mut modal.field).desired_width(160.0));
            ui.end_row();
        });
        if let Some(error) = &modal.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(crate::widgets::button::primary(ui, "Move")).clicked() {
                cmds.push(AppCommand::MoveToCredentials {
                    env_id: modal.env_id.clone(),
                    var_idx: modal.var_idx,
                    entry: modal.entry.trim().to_string(),
                    field: modal.field.trim().to_string(),
                });
            }
            if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close = true;
            }
        });
    });
    if close {
        state.move_modal = None;
    }
}

fn variant_modal(ctx: &egui::Context, state: &mut EnvironmentsState, env_idx: usize, now: f64) {
    let Some(modal_state) = &mut state.variant_modal else { return; };
    let mut close = false;
    let mut commit = false;

    egui::Modal::new(egui::Id::new("variant_modal")).show(ctx, |ui| {
        ui.set_width(260.0);
        let title = if modal_state.is_rename { "Rename Variant" } else { "Add Variant" };
        ui.heading(title);
        ui.add_space(6.0);
        let resp = ui.add(
            egui::TextEdit::singleline(&mut modal_state.name).hint_text("Variant name"),
        );
        let name = modal_state.name.trim().to_string();
        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if enter && !name.is_empty() {
            commit = true;
        } else {
            resp.request_focus();
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            close = true;
        }
        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let can_commit = !name.is_empty();
            if ui.add_enabled(can_commit, crate::widgets::button::primary(ui, if modal_state.is_rename { "Rename" } else { "Add" })).clicked() {
                commit = true;
            }
            if ui.button("Cancel").clicked() {
                close = true;
            }
        });
    });

    if commit {
        if let Some(modal) = state.variant_modal.take() {
            let name = modal.name.trim().to_string();
            if !name.is_empty() && env_idx < state.environments.len() {
                let env = &mut state.environments[env_idx];
                let env_id = env.id.clone();
                if modal.var_idx < env.variables.len() {
                    let var = &mut env.variables[modal.var_idx];
                    if modal.is_rename {
                        if !var.variants.is_empty() {
                            let idx = var.active_index.min(var.variants.len() - 1);
                            var.variants[idx].name = name;
                        }
                    } else if var.variants.is_empty() {
                        var.variants.push(VariableVariant {
                            name: "default".to_string(),
                            value: var.value.clone(),
                        });
                        var.variants.push(VariableVariant {
                            name,
                            value: String::new(),
                        });
                        var.active_index = 1;
                        var.value = String::new();
                    } else {
                        var.variants.push(VariableVariant {
                            name,
                            value: String::new(),
                        });
                        var.active_index = var.variants.len() - 1;
                        var.value = String::new();
                    }
                    state.mark_dirty(&env_id, now);
                }
            }
        }
    } else if close {
        state.variant_modal = None;
    }
}
