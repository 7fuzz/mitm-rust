mod commands;
mod export;
mod import;
mod quick;
mod view;

use mitm_core::repeater::REPEATER_WORKSPACE_ID;
use mitm_core::workspace::{Environment, Workspace};
use crate::util::save_queue::SaveQueue;
use mitm_core::db::Db;

pub use commands::{handle, Cmd};
pub use export::show as show_export;
pub use import::{file_label, is_json_file, show as show_import};
pub use quick::{show as show_quick_vars, Scope as QuickVarScope};
pub use view::show;

pub struct VariantModal {
    pub var_idx: usize,
    pub is_rename: bool,
    pub name: String,
}

pub struct MoveModal {
    pub env_id: String,
    pub var_idx: usize,
    pub vault: String,
    pub entry: String,
    pub field: String,
    pub error: Option<String>,
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
    pub move_modal: Option<MoveModal>,
    pub renaming_workspace: bool,
    pub rename_workspace_name: String,
    pub editing_description: bool,
    pub description_text: String,
    pub confirm_delete_workspace: bool,
    pub quick: quick::QuickVars,
    pub import: import::ImportState,
    pub export: export::ExportPrompt,
    saves: SaveQueue,
    /// Failed writes outside the save queue, reported with the save errors.
    errors: Vec<String>,
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
            move_modal: None,
            renaming_workspace: false,
            rename_workspace_name: String::new(),
            editing_description: false,
            description_text: String::new(),
            confirm_delete_workspace: false,
            quick: quick::QuickVars::default(),
            import: import::ImportState::default(),
            export: export::ExportPrompt::default(),
            saves: SaveQueue::default(),
            errors: Vec::new(),
        }
    }
}

impl EnvironmentsState {
    pub fn load(&mut self, db: &Db) {
        self.workspaces = mitm_core::workspace::get_workspaces_db(db).unwrap_or_default();
        if self.workspaces.is_empty() {
            if let Ok(created) = mitm_core::workspace::create_workspace_db(
                db,
                "Default Workspace".to_string(),
                Some("Main development workspace".to_string()),
            ) {
                self.workspaces.push(created);
            }
        }
        if let Ok(Some(active)) = mitm_core::workspace::get_active_workspace_db(db) {
            self.active_workspace_id = active.id;
        } else if let Some(first) = self.workspaces.first() {
            self.active_workspace_id = first.id.clone();
            if let Err(e) = mitm_core::workspace::set_active_workspace_db(db, &first.id) {
                self.errors.push(format!("Couldn't select workspace: {e}"));
            }
        }
        self.reload_environments(db);
        self.load_repeater_env(db);
    }

    fn load_repeater_env(&mut self, db: &Db) {
        let envs = mitm_core::workspace::get_workspace_environments_db(db, REPEATER_WORKSPACE_ID).unwrap_or_default();
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
            mitm_core::workspace::save_workspace_environment_db(db, env.clone()).ok().map(|_| env)
        });
    }

    pub fn reload_environments(&mut self, db: &Db) {
        self.environments = mitm_core::workspace::get_workspace_environments_db(db, &self.active_workspace_id).unwrap_or_default();
        self.active_environment_id = self.environments.iter().find(|e| e.is_active).map(|e| e.id.clone());
        if self.selected_environment_id.is_none() || !self.environments.iter().any(|e| Some(&e.id) == self.selected_environment_id.as_ref()) {
            self.selected_environment_id = self.active_environment_id.clone().or_else(|| self.environments.first().map(|e| e.id.clone()));
        }
    }

    /// Picks up variables a request run extracted into the database.
    pub fn refresh(&mut self, db: &Db) {
        self.flush(db);
        self.reload_environments(db);
        self.load_repeater_env(db);
    }

    pub fn active_workspace(&self) -> Option<&Workspace> {
        self.workspaces.iter().find(|w| w.id == self.active_workspace_id)
    }

    pub fn active_environment(&self) -> Option<&Environment> {
        let active_id = self.active_environment_id.as_ref()?;
        self.environments.iter().find(|e| &e.id == active_id)
    }

    pub fn mark_dirty(&mut self, env_id: &str, now: f64) {
        self.saves.edited(env_id, now);
    }

    pub fn flush_if_idle(&mut self, db: &Db, now: f64) {
        if self.saves.is_due(now) {
            self.flush(db);
            self.saves.retry_later(now);
        }
    }

    pub fn take_save_errors(&mut self) -> Vec<String> {
        let mut errors = std::mem::take(&mut self.errors);
        errors.extend(self.saves.take_errors());
        errors
    }

    pub fn flush(&mut self, db: &Db) {
        for id in self.saves.take() {
            let Some(env) = self.environments.iter().chain(&self.repeater_env).find(|e| e.id == id).cloned() else { continue };
            match mitm_core::workspace::save_workspace_environment_db(db, env) {
                Ok(_) => self.saves.saved(&id),
                Err(e) => self.saves.failed(id, format!("Couldn't save environment: {e}")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::view::field_key_from;
    use mitm_core::workspace::{EnvironmentVariable, VariableVariant};

    #[test]
    fn variable_keys_become_valid_field_keys() {
        assert_eq!(field_key_from("adminPassword"), "adminPassword");
        assert_eq!(field_key_from(" admin.pass word "), "admin_pass_word");
        assert_eq!(field_key_from(""), "value");
    }

    #[test]
    fn repeater_gets_one_default_environment_apart_from_workspaces() {
        let dir = std::env::temp_dir().join(format!("mitm-repeater-env-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = mitm_core::db::Db::new(dir.join("test.db"));
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
