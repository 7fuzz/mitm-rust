use crate::{workspace, MitmCore};

impl MitmCore {
    pub fn get_workspaces(&self) -> Result<Vec<workspace::Workspace>, String> {
        workspace::get_workspaces_db(&self.db_path)
    }

    pub fn get_workspace(&self, id: &str) -> Result<Option<workspace::Workspace>, String> {
        workspace::get_workspace_db(&self.db_path, id)
    }

    pub fn get_active_workspace(&self) -> Result<Option<workspace::Workspace>, String> {
        workspace::get_active_workspace_db(&self.db_path)
    }

    pub fn set_active_workspace(&self, id: &str) -> Result<(), String> {
        workspace::set_active_workspace_db(&self.db_path, id)
    }

    pub fn create_workspace(&self, name: &str, description: Option<&str>) -> Result<workspace::Workspace, String> {
        workspace::create_workspace_db(&self.db_path, name.to_string(), description.map(|s| s.to_string()))
    }

    pub fn rename_workspace(&self, id: &str, new_name: &str) -> Result<(), String> {
        workspace::rename_workspace_db(&self.db_path, id, new_name)
    }

    pub fn update_workspace_description(&self, id: &str, description: Option<&str>) -> Result<(), String> {
        workspace::update_workspace_description_db(&self.db_path, id, description)
    }

    pub fn delete_workspace(&self, id: &str) -> Result<(), String> {
        workspace::delete_workspace_db(&self.db_path, id)
    }

    pub fn get_workspace_environments(&self, workspace_id: &str) -> Result<Vec<workspace::Environment>, String> {
        workspace::get_workspace_environments_db(&self.db_path, workspace_id)
    }

    pub fn get_workspace_environment(&self, id: &str) -> Result<Option<workspace::Environment>, String> {
        workspace::get_workspace_environment_db(&self.db_path, id)
    }

    pub fn save_workspace_environment(&self, env: workspace::Environment) -> Result<(), String> {
        workspace::save_workspace_environment_db(&self.db_path, env)
    }

    pub fn delete_workspace_environment(&self, id: &str) -> Result<(), String> {
        workspace::delete_workspace_environment_db(&self.db_path, id)
    }

    pub fn duplicate_workspace_environment(&self, id: &str, new_name: Option<&str>) -> Result<workspace::Environment, String> {
        workspace::duplicate_workspace_environment_db(&self.db_path, id, new_name)
    }

    pub fn set_active_environment(&self, workspace_id: &str, env_id: Option<&str>) -> Result<(), String> {
        workspace::set_active_environment_db(&self.db_path, workspace_id, env_id)
    }

    pub fn get_active_environment(&self) -> Result<Option<workspace::Environment>, String> {
        if let Some(ws) = self.get_active_workspace()? {
            let envs = self.get_workspace_environments(&ws.id)?;
            if let Some(active_id) = &ws.active_environment_id {
                if let Some(env) = envs.iter().find(|e| &e.id == active_id) {
                    return Ok(Some(env.clone()));
                }
            }
            if let Some(env) = envs.into_iter().find(|e| e.is_active) {
                return Ok(Some(env));
            }
        }
        Ok(None)
    }

    pub fn resolve_vars(&self, text: &str) -> (String, Vec<workspace::Unresolved>) {
        let active_env = self.get_active_environment().ok().flatten();
        let workspace_id = self.get_active_workspace().ok().flatten().map(|w| w.id);
        let scope = workspace_id
            .as_deref()
            .map(|workspace_id| workspace::CredentialScope { db_path: &self.db_path, workspace_id });
        let resolved = workspace::interpolate(active_env.as_ref(), scope, text, &mut Default::default());
        let unresolved = workspace::find_unresolved_variables(&resolved);
        (resolved, unresolved)
    }

    pub fn variable_value(&self, name: &str) -> Option<String> {
        let active_env = self.get_active_environment().ok().flatten();
        workspace::variable_value_in_env(active_env.as_ref(), name)
    }
}
