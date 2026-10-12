use mitm_core::workspace::{self as core_workspace, Environment};
use crate::app::command::AppCommand;
use crate::app::cx::Cx;
use super::EnvironmentsState;

#[derive(Clone)]
pub enum Cmd {
    Select(Option<String>),
    Create { workspace_id: String, name: String },
    Rename { id: String, name: String },
    Duplicate(String),
    Delete(String),
    RenameWorkspace { id: String, name: String },
    DescribeWorkspace { id: String, description: Option<String> },
}

impl Cmd {
    /// Commands that reload environments from the DB, so pending edits must be saved first.
    pub fn reloads(&self) -> bool {
        !matches!(self, Cmd::RenameWorkspace { .. } | Cmd::DescribeWorkspace { .. })
    }
}

impl From<Cmd> for AppCommand {
    fn from(cmd: Cmd) -> Self {
        AppCommand::Environments(cmd)
    }
}

pub fn handle(state: &mut EnvironmentsState, cx: &Cx, cmd: Cmd) {
    let db_path = cx.db_path();
    match cmd {
        Cmd::Select(env_id) => {
            let ws_id = state.active_workspace_id.clone();
            cx.report(core_workspace::set_active_environment_db(db_path, &ws_id, env_id.as_deref()));
            state.reload_environments(db_path);
        }
        Cmd::Create { workspace_id, name } => {
            let now = chrono::Utc::now().timestamp_millis();
            let env = Environment {
                id: uuid::Uuid::new_v4().to_string(),
                workspace_id,
                name,
                is_active: false,
                variables: Vec::new(),
                created_at_ms: now,
                updated_at_ms: now,
            };
            let id = env.id.clone();
            cx.report(core_workspace::save_workspace_environment_db(db_path, env));
            state.reload_environments(db_path);
            state.selected_environment_id = Some(id);
        }
        Cmd::Rename { id, name } => {
            if let Some(Some(mut env)) = cx.report(core_workspace::get_workspace_environment_db(db_path, &id)) {
                env.name = name;
                cx.report(core_workspace::save_workspace_environment_db(db_path, env));
                state.reload_environments(db_path);
            }
        }
        Cmd::Duplicate(id) => {
            if let Some(dup) = cx.report(core_workspace::duplicate_workspace_environment_db(db_path, &id, None)) {
                state.reload_environments(db_path);
                state.selected_environment_id = Some(dup.id);
            }
        }
        Cmd::Delete(id) => {
            cx.report(core_workspace::delete_workspace_environment_db(db_path, &id));
            state.reload_environments(db_path);
            if state.selected_environment_id.as_deref() == Some(&id) {
                state.selected_environment_id = state.environments.first().map(|e| e.id.clone());
            }
        }
        Cmd::RenameWorkspace { id, name } => {
            if cx.report(core_workspace::rename_workspace_db(db_path, &id, &name)).is_some() {
                if let Some(ws) = state.workspaces.iter_mut().find(|w| w.id == id) {
                    ws.name = name;
                }
            }
        }
        Cmd::DescribeWorkspace { id, description } => {
            if cx.report(core_workspace::update_workspace_description_db(db_path, &id, description.as_deref())).is_some() {
                if let Some(ws) = state.workspaces.iter_mut().find(|w| w.id == id) {
                    ws.description = description;
                }
            }
        }
    }
}
