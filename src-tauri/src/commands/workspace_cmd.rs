use tauri::State;
use crate::state::AppState;
use crate::workspace::{
    create_workspace_db, delete_workspace_db, delete_workspace_environment_db, get_workspace_environments_db,
    get_workspaces_db, save_workspace_environment_db, set_active_workspace_db,
    update_workspace_db, Environment, Workspace,
};
use crate::workspace::import::{import_workspace_json_db, ImportSummary};
use crate::workspace::export::{export_workspace_json_db, export_workspace_file_db};

#[tauri::command]
pub async fn get_workspaces(
    state: State<'_, AppState>,
) -> Result<Vec<Workspace>, String> {
    get_workspaces_db(&state.db_path)
}

#[tauri::command]
pub async fn create_workspace(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
) -> Result<Workspace, String> {
    create_workspace_db(&state.db_path, name, description)
}

#[tauri::command]
pub async fn update_workspace(
    state: State<'_, AppState>,
    workspace: Workspace,
) -> Result<(), String> {
    update_workspace_db(&state.db_path, workspace)
}

#[tauri::command]
pub async fn delete_workspace(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    delete_workspace_db(&state.db_path, &id)
}

#[tauri::command]
pub async fn set_active_workspace(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    set_active_workspace_db(&state.db_path, &id)
}

#[tauri::command]
pub async fn import_workspace_json(
    state: State<'_, AppState>,
    json_content: String,
    target_workspace_id: Option<String>,
    custom_workspace_name: Option<String>,
) -> Result<ImportSummary, String> {
    import_workspace_json_db(
        &state.db_path,
        &json_content,
        target_workspace_id.as_deref(),
        custom_workspace_name.as_deref(),
    )
}

#[tauri::command]
pub async fn get_workspace_environments(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<Vec<Environment>, String> {
    get_workspace_environments_db(&state.db_path, &workspace_id)
}

#[tauri::command]
pub async fn delete_workspace_environment(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    delete_workspace_environment_db(&state.db_path, &id)
}

#[tauri::command]
pub async fn save_workspace_environment(
    state: State<'_, AppState>,
    environment: Environment,
) -> Result<(), String> {
    save_workspace_environment_db(&state.db_path, environment)
}

#[tauri::command]
pub async fn export_workspace_json(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<String, String> {
    export_workspace_json_db(&state.db_path, &workspace_id)
}

#[tauri::command]
pub async fn export_workspace_file(
    state: State<'_, AppState>,
    workspace_id: String,
    destination_path: String,
) -> Result<(), String> {
    export_workspace_file_db(&state.db_path, &workspace_id, &destination_path)
}

