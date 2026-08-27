use tauri::State;
use crate::state::AppState;
use crate::collections::{
    clear_request_histories_db, create_collection_db, create_request_db, delete_collection_db,
    delete_request_db, duplicate_collection_db, duplicate_request_db, get_collections_db,
    get_request_histories_db, move_collection_db, move_request_db, update_collection_db,
    update_request_db, Collection, CollectionTreeItem, RequestHistoryItem, RequestItem,
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
pub async fn move_collection(
    state: State<'_, AppState>,
    collection_id: String,
    target_parent_id: Option<String>,
) -> Result<(), String> {
    move_collection_db(&state.db_path, &collection_id, target_parent_id)
}

#[tauri::command]
pub async fn duplicate_collection(
    state: State<'_, AppState>,
    collection_id: String,
) -> Result<String, String> {
    duplicate_collection_db(&state.db_path, &collection_id)
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
pub async fn move_request(
    state: State<'_, AppState>,
    request_id: String,
    target_collection_id: String,
) -> Result<(), String> {
    move_request_db(&state.db_path, &request_id, &target_collection_id)
}

#[tauri::command]
pub async fn duplicate_request(
    state: State<'_, AppState>,
    request_id: String,
) -> Result<RequestItem, String> {
    duplicate_request_db(&state.db_path, &request_id)
}

#[tauri::command]
pub async fn execute_collection_request(
    state: State<'_, AppState>,
    request_id: String,
) -> Result<ExecutionResult, String> {
    execute_collection_request_db(&state.db_path, &request_id).await
}

#[tauri::command]
pub async fn get_request_histories(
    state: State<'_, AppState>,
    request_id: String,
) -> Result<Vec<RequestHistoryItem>, String> {
    get_request_histories_db(&state.db_path, &request_id)
}

#[tauri::command]
pub async fn clear_request_histories(
    state: State<'_, AppState>,
    request_id: String,
) -> Result<(), String> {
    clear_request_histories_db(&state.db_path, &request_id)
}

#[tauri::command]
pub async fn read_file_as_base64(file_path: String) -> Result<String, String> {
    use base64::Engine;
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "json" => "application/json",
        "txt" => "text/plain",
        "html" => "text/html",
        _ => "application/octet-stream",
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{};base64,{}", mime, encoded))
}
