use tauri::State;
use crate::state::AppState;
use crate::collections::{
    create_collection_db, create_request_db, delete_collection_db, delete_request_db,
    get_collections_db, update_collection_db, update_request_db, Collection, CollectionTreeItem,
    RequestItem,
};
use crate::collections::execute::{execute_collection_request_db, ExecutionResult};

#[tauri::command]
pub async fn get_collections(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<Vec<CollectionTreeItem>, String> {
    get_collections_db(&state.db_path, &workspace_id)
}

#[tauri::command]
pub async fn create_collection(
    state: State<'_, AppState>,
    workspace_id: String,
    parent_id: Option<String>,
    name: String,
) -> Result<Collection, String> {
    create_collection_db(&state.db_path, workspace_id, parent_id, name)
}

#[tauri::command]
pub async fn update_collection(
    state: State<'_, AppState>,
    collection: Collection,
) -> Result<(), String> {
    update_collection_db(&state.db_path, collection)
}

#[tauri::command]
pub async fn delete_collection(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    delete_collection_db(&state.db_path, &id)
}

#[tauri::command]
pub async fn create_request(
    state: State<'_, AppState>,
    collection_id: String,
    name: String,
) -> Result<RequestItem, String> {
    create_request_db(&state.db_path, collection_id, name)
}

#[tauri::command]
pub async fn update_request(
    state: State<'_, AppState>,
    request: RequestItem,
) -> Result<(), String> {
    update_request_db(&state.db_path, request)
}

#[tauri::command]
pub async fn delete_request(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    delete_request_db(&state.db_path, &id)
}

#[tauri::command]
pub async fn execute_collection_request(
    state: State<'_, AppState>,
    request_id: String,
) -> Result<ExecutionResult, String> {
    execute_collection_request_db(&state.db_path, &request_id).await
}
