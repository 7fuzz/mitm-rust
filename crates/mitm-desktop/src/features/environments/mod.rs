use std::path::Path;
use eframe::egui;
use mitm_core::workspace::{Environment, Workspace};
use crate::command::Commands;

#[derive(Default)]
pub struct EnvironmentsState {
    pub workspaces: Vec<Workspace>,
    pub active_workspace_id: String,
    pub environments: Vec<Environment>,
    pub active_environment_id: Option<String>,
    pub selected_environment_id: Option<String>,
    pub workspace_search: String,
    pub new_workspace_modal: bool,
    pub new_workspace_name: String,
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
}

pub fn show(ui: &mut egui::Ui, state: &mut EnvironmentsState, _cmds: &mut Commands, _db_path: &Path) {
    ui.vertical(|ui| {
        let ws_name = state.active_workspace().map_or("None", |w| &w.name);
        ui.heading(format!("Workspace: {}", ws_name));
        ui.separator();
        ui.label(format!("{} environment(s)", state.environments.len()));
    });
}
