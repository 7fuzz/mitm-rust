mod import;
mod quick;

use std::collections::HashSet;
use std::path::Path;
use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use mitm_core::repeater::REPEATER_WORKSPACE_ID;
use mitm_core::workspace::{Environment, EnvironmentVariable, VariableVariant, Workspace};
use crate::command::{AppCommand, Commands};
use crate::widgets::split;

const SAVE_DELAY_SECS: f64 = 0.6;

pub use import::{file_label, is_json_file, show as show_import};
pub use quick::{show as show_quick_vars, Scope as QuickVarScope};

pub struct VariantModal {
    pub var_idx: usize,
    pub is_rename: bool,
    pub name: String,
}

pub struct EnvironmentsState {
    pub workspaces: Vec<Workspace>,
    pub active_workspace_id: String,
    pub environments: Vec<Environment>,
    pub active_environment_id: Option<String>,
    /// Repeater is a scratchpad with its own environment, kept apart from the workspaces.
    pub repeater_env: Option<Environment>,
    pub selected_environment_id: Option<String>,
    pub split_fraction: f32,
    pub renaming_env_id: Option<String>,
    pub rename_text: String,
    pub workspace_search: String,
    pub new_workspace_modal: bool,
    pub new_workspace_name: String,
    pub variable_filter: String,
    pub new_var_key: String,
    pub new_var_value: String,
    pub variant_modal: Option<VariantModal>,
    pub renaming_workspace: bool,
    pub rename_workspace_name: String,
    pub editing_description: bool,
    pub description_text: String,
    pub confirm_delete_workspace: bool,
    pub quick: quick::QuickVars,
    pub import: import::ImportState,
    dirty: HashSet<String>,
    last_edit: f64,
}

impl Default for EnvironmentsState {
    fn default() -> Self {
        Self {
            workspaces: Vec::new(),
            active_workspace_id: String::new(),
            environments: Vec::new(),
            active_environment_id: None,
            repeater_env: None,
            selected_environment_id: None,
            split_fraction: 0.25,
            renaming_env_id: None,
            rename_text: String::new(),
            workspace_search: String::new(),
            new_workspace_modal: false,
            new_workspace_name: String::new(),
            variable_filter: String::new(),
            new_var_key: String::new(),
            new_var_value: String::new(),
            variant_modal: None,
            renaming_workspace: false,
            rename_workspace_name: String::new(),
            editing_description: false,
            description_text: String::new(),
            confirm_delete_workspace: false,
            quick: quick::QuickVars::default(),
            import: import::ImportState::default(),
            dirty: HashSet::new(),
            last_edit: 0.0,
        }
    }
}

impl EnvironmentsState {
    pub fn load(&mut self, db_path: &Path) {
        let db_buf = db_path.to_path_buf();
        self.workspaces = mitm_core::workspace::get_workspaces_db(&db_buf).unwrap_or_default();
        if self.workspaces.is_empty() {
            if let Ok(created) = mitm_core::workspace::create_workspace_db(
                &db_buf,
                "Default Workspace".to_string(),
                Some("Main development workspace".to_string()),
            ) {
                self.workspaces.push(created);
            }
        }
        if let Ok(Some(active)) = mitm_core::workspace::get_active_workspace_db(&db_buf) {
            self.active_workspace_id = active.id;
        } else if let Some(first) = self.workspaces.first() {
            self.active_workspace_id = first.id.clone();
            let _ = mitm_core::workspace::set_active_workspace_db(&db_buf, &first.id);
        }
        self.reload_environments(db_path);
        self.load_repeater_env(db_path);
    }

    fn load_repeater_env(&mut self, db_path: &Path) {
        let db_buf = db_path.to_path_buf();
        let envs = mitm_core::workspace::get_workspace_environments_db(&db_buf, REPEATER_WORKSPACE_ID).unwrap_or_default();
        let existing = envs.iter().find(|e| e.is_active).or(envs.first()).cloned();
        self.repeater_env = existing.or_else(|| {
            let now = chrono::Utc::now().timestamp_millis();
            let env = Environment {
                id: uuid::Uuid::new_v4().to_string(),
                workspace_id: REPEATER_WORKSPACE_ID.to_string(),
                name: "Default".to_string(),
                is_active: true,
                variables: Vec::new(),
                created_at_ms: now,
                updated_at_ms: now,
            };
            mitm_core::workspace::save_workspace_environment_db(&db_buf, env.clone()).ok().map(|_| env)
        });
    }

    pub fn reload_environments(&mut self, db_path: &Path) {
        let db_buf = db_path.to_path_buf();
        self.environments = mitm_core::workspace::get_workspace_environments_db(&db_buf, &self.active_workspace_id).unwrap_or_default();
        self.active_environment_id = self.environments.iter().find(|e| e.is_active).map(|e| e.id.clone());
        if self.selected_environment_id.is_none() || !self.environments.iter().any(|e| Some(&e.id) == self.selected_environment_id.as_ref()) {
            self.selected_environment_id = self.active_environment_id.clone().or_else(|| self.environments.first().map(|e| e.id.clone()));
        }
    }

    /// Picks up variables a request run extracted into the database.
    pub fn refresh(&mut self, db_path: &Path) {
        self.flush(db_path);
        self.reload_environments(db_path);
        self.load_repeater_env(db_path);
    }

    pub fn active_workspace(&self) -> Option<&Workspace> {
        self.workspaces.iter().find(|w| w.id == self.active_workspace_id)
    }

    pub fn active_environment(&self) -> Option<&Environment> {
        let active_id = self.active_environment_id.as_ref()?;
        self.environments.iter().find(|e| &e.id == active_id)
    }

    pub fn mark_dirty(&mut self, env_id: &str, now: f64) {
        self.dirty.insert(env_id.to_string());
        self.last_edit = now;
    }

    pub fn flush_if_idle(&mut self, db_path: &Path, now: f64) {
        if !self.dirty.is_empty() && now - self.last_edit >= SAVE_DELAY_SECS {
            self.flush(db_path);
        }
    }

    pub fn flush(&mut self, db_path: &Path) {
        let db_buf = db_path.to_path_buf();
        for id in std::mem::take(&mut self.dirty) {
            if let Some(env) = self.environments.iter().chain(&self.repeater_env).find(|e| e.id == id).cloned() {
                let _ = mitm_core::workspace::save_workspace_environment_db(&db_buf, env);
            }
        }
    }
}

pub fn show(ui: &mut egui::Ui, state: &mut EnvironmentsState, cmds: &mut Commands) {
    let now = ui.input(|i| i.time);
    workspace_header(ui, state, cmds);
    ui.add_space(4.0);
    ui.separator();
    let [mut list_ui, mut editor_ui] = split::side_by_side(ui, "environments_split", &mut state.split_fraction);
    environment_list(&mut list_ui, state, cmds);
    environment_editor(&mut editor_ui, state, now);
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
                    cmds.push(AppCommand::RenameWorkspace {
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
            let del_btn = egui::Button::new("🗑 Delete workspace");
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
                .desired_width(ui.available_width() - 80.0),
        );
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            state.editing_description = false;
        } else if resp.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let trimmed = state.description_text.trim();
            let new_desc = if trimmed.is_empty() { None } else { Some(trimmed.to_string()) };
            if new_desc != ws.description {
                cmds.push(AppCommand::UpdateWorkspaceDescription {
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
            RichText::new("Add a description…").weak().italics()
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
            ui.set_width(320.0);
            ui.heading("Delete Workspace");
            ui.add_space(6.0);
            ui.label(format!("Are you sure you want to delete \"{}\"?", ws.name));
            ui.weak("All collections and environments in this workspace will be deleted.");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Delete").color(ui.visuals().error_fg_color)).clicked() {
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
    ui.horizontal(|ui| {
        ui.strong("Environments");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("➕").on_hover_text("Add environment").clicked() {
                cmds.push(AppCommand::CreateEnvironment {
                    workspace_id: state.active_workspace_id.clone(),
                    name: "New Environment".to_string(),
                });
            }
        });
    });
    ui.separator();

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
                let bullet = if is_active { "● " } else { "○ " };
                let label = format!("{}{}", bullet, env.name);
                let resp = ui.selectable_label(is_selected, label);

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
                            cmds.push(AppCommand::SelectEnvironment(None));
                            ui.close();
                        }
                    } else if ui.button("Set active").clicked() {
                        cmds.push(AppCommand::SelectEnvironment(Some(env.id.clone())));
                        ui.close();
                    }
                    if ui.button("Rename").clicked() {
                        state.renaming_env_id = Some(env.id.clone());
                        state.rename_text = env.name.clone();
                        ui.close();
                    }
                    if ui.button("Duplicate").clicked() {
                        cmds.push(AppCommand::DuplicateEnvironment(env.id.clone()));
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Delete").clicked() {
                        cmds.push(AppCommand::DeleteEnvironment(env.id.clone()));
                        ui.close();
                    }
                });
            }
        }

        if let Some((id, name)) = rename_commit {
            cmds.push(AppCommand::RenameEnvironment { id, name });
            state.renaming_env_id = None;
            state.rename_text.clear();
        }
        if rename_cancel {
            state.renaming_env_id = None;
            state.rename_text.clear();
        }
    });
}

fn environment_editor(ui: &mut egui::Ui, state: &mut EnvironmentsState, now: f64) {
    let Some(selected_id) = state.selected_environment_id.clone() else { return };
    let Some(env_idx) = state.environments.iter().position(|e| e.id == selected_id) else { return };

    let env = &mut state.environments[env_idx];

    ui.horizontal(|ui| {
        ui.heading(&env.name);
        if env.is_active {
            ui.label(RichText::new("● Active").color(ui.visuals().selection.bg_fill).strong());
        }
    });
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.variable_filter)
                .hint_text("Filter variables…")
                .desired_width(ui.available_width() - 80.0),
        );
        if !state.variable_filter.is_empty() && ui.button("Clear").clicked() {
            state.variable_filter.clear();
        }
    });
    ui.separator();

    let filter = state.variable_filter.trim().to_lowercase();
    let mut changed = false;
    let mut delete_idx = None;
    let mut open_variant_modal = None;
    let mut delete_variant_idx = None;

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
        .striped(true)
        .resizable(true)
        .auto_shrink([false, false])
        .column(Column::exact(28.0))
        .column(Column::exact(28.0))
        .column(Column::initial(160.0).at_least(80.0).clip(true))
        .column(Column::remainder().at_least(120.0).clip(true))
        .column(Column::exact(28.0))
        .header(22.0, |mut header| {
            header.col(|ui| { ui.strong(""); });
            header.col(|ui| { ui.strong(""); });
            header.col(|ui| { ui.strong("Key"); });
            header.col(|ui| { ui.strong("Value"); });
            header.col(|ui| { ui.strong(""); });
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
                        let icon = if is_secret { "🔒" } else { "🔓" };
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
                                    if ui.button("➕ Add variant…").clicked() {
                                        open_variant_modal = Some((var_idx, false));
                                        ui.close();
                                    }
                                    if ui.button("✏ Rename variant…").clicked() {
                                        open_variant_modal = Some((var_idx, true));
                                        ui.close();
                                    }
                                    if ui.button("🗑 Delete variant").clicked() {
                                        delete_variant_idx = Some((var_idx, active_idx));
                                        ui.close();
                                    }
                                });
                            }

                            let is_secret = var.r#type == "secret";
                            let mut revealed = false;
                            if is_secret {
                                let eye = ui.add(egui::Button::new("👁").sense(egui::Sense::drag()))
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
                                if ui.button("➕ Add variant…").clicked() {
                                    open_variant_modal = Some((var_idx, false));
                                    ui.close();
                                }
                                if !var.variants.is_empty() {
                                    let active_idx = var.active_index.min(var.variants.len() - 1);
                                    if ui.button("✏ Rename variant…").clicked() {
                                        open_variant_modal = Some((var_idx, true));
                                        ui.close();
                                    }
                                    if ui.button("🗑 Delete variant").clicked() {
                                        delete_variant_idx = Some((var_idx, active_idx));
                                        ui.close();
                                    }
                                }
                            });
                        });
                    });
                    row.col(|ui| {
                        if ui.small_button("✖").on_hover_text("Delete variable").clicked() {
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

    variant_modal(ui.ctx(), state, env_idx, now);
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
        ui.horizontal(|ui| {
            let can_commit = !name.is_empty();
            if ui.add_enabled(can_commit, egui::Button::new(if modal_state.is_rename { "Rename" } else { "Add" })).clicked() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeater_gets_one_default_environment_apart_from_workspaces() {
        let dir = std::env::temp_dir().join(format!("mitm-repeater-env-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("test.db");
        mitm_core::db::init_database(&db, None).unwrap();

        let mut state = EnvironmentsState::default();
        state.load(&db);
        let repeater = state.repeater_env.clone().expect("repeater env created");
        assert_eq!(repeater.workspace_id, REPEATER_WORKSPACE_ID);
        assert!(state.environments.iter().all(|e| e.id != repeater.id));

        state.load(&db);
        assert_eq!(state.repeater_env.unwrap().id, repeater.id);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn variant_cycling_and_current_value() {
        let mut var = EnvironmentVariable {
            key: "host".to_string(),
            value: "http://localhost".to_string(),
            enabled: true,
            r#type: "default".to_string(),
            active_index: 0,
            variants: vec![
                VariableVariant {
                    name: "local".to_string(),
                    value: "http://localhost".to_string(),
                },
                VariableVariant {
                    name: "prod".to_string(),
                    value: "https://api.example.com".to_string(),
                },
            ],
        };
        assert_eq!(var.current_value(), "http://localhost");
        var.active_index = (var.active_index + 1) % var.variants.len();
        assert_eq!(var.current_value(), "https://api.example.com");
    }
}
