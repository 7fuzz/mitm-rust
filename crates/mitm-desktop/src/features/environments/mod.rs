use std::collections::HashSet;
use std::path::Path;
use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use mitm_core::workspace::{Environment, EnvironmentVariable, Workspace};
use crate::command::{AppCommand, Commands};
use crate::widgets::split;

const SAVE_DELAY_SECS: f64 = 0.6;

pub struct EnvironmentsState {
    pub workspaces: Vec<Workspace>,
    pub active_workspace_id: String,
    pub environments: Vec<Environment>,
    pub active_environment_id: Option<String>,
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
    }

    pub fn reload_environments(&mut self, db_path: &Path) {
        let db_buf = db_path.to_path_buf();
        self.environments = mitm_core::workspace::get_workspace_environments_db(&db_buf, &self.active_workspace_id).unwrap_or_default();
        self.active_environment_id = self.environments.iter().find(|e| e.is_active).map(|e| e.id.clone());
        if self.selected_environment_id.is_none() || !self.environments.iter().any(|e| Some(&e.id) == self.selected_environment_id.as_ref()) {
            self.selected_environment_id = self.active_environment_id.clone().or_else(|| self.environments.first().map(|e| e.id.clone()));
        }
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
            if let Some(env) = self.environments.iter().find(|e| e.id == id).cloned() {
                let _ = mitm_core::workspace::save_workspace_environment_db(&db_buf, env);
            }
        }
    }
}

pub fn show(ui: &mut egui::Ui, state: &mut EnvironmentsState, cmds: &mut Commands, _db_path: &Path) {
    let now = ui.input(|i| i.time);
    let [mut list_ui, mut editor_ui] = split::side_by_side(ui, "environments_split", &mut state.split_fraction);
    environment_list(&mut list_ui, state, cmds);
    environment_editor(&mut editor_ui, state, cmds, now);
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
                resp.request_focus();
                if resp.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let text = state.rename_text.trim().to_string();
                    if !text.is_empty() {
                        rename_commit = Some((env.id.clone(), text));
                    } else {
                        rename_cancel = true;
                    }
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    rename_cancel = true;
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

fn environment_editor(ui: &mut egui::Ui, state: &mut EnvironmentsState, _cmds: &mut Commands, now: f64) {
    let Some(selected_id) = state.selected_environment_id.clone() else {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.weak("Select an environment from the list or add a new one");
        });
        return;
    };

    let Some(env_idx) = state.environments.iter().position(|e| e.id == selected_id) else {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.weak("Environment not found");
        });
        return;
    };

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
        .column(Column::exact(30.0))
        .column(Column::initial(180.0).at_least(80.0).clip(true))
        .column(Column::remainder().at_least(120.0).clip(true))
        .column(Column::exact(28.0))
        .header(22.0, |mut header| {
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
                        changed |= ui.add(
                            egui::TextEdit::singleline(&mut var.key)
                                .hint_text("Key")
                                .desired_width(ui.available_width()),
                        ).changed();
                    });
                    row.col(|ui| {
                        changed |= ui.add(
                            egui::TextEdit::singleline(&mut var.value)
                                .hint_text("Value")
                                .desired_width(ui.available_width()),
                        ).changed();
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

    if let Some(idx) = delete_idx {
        env.variables.remove(idx);
        changed = true;
    }

    if changed {
        state.mark_dirty(&selected_id, now);
    }
}
