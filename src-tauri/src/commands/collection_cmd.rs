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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RequestPreview {
    pub method: String,
    pub url: String,
    pub host: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub body_type: String,
    pub full_url_request: String,
    pub full_request: String,
    pub curl_command: String,
}

#[tauri::command]
pub async fn preview_collection_request(
    state: State<'_, AppState>,
    request_id: String,
) -> Result<RequestPreview, String> {
    use crate::workspace::interpolate_variables;
    use rusqlite::Connection;

    let db_path = &state.db_path;
    let (method, raw_url, headers_json, params_json, body_type, b_json, b_raw, b_form, b_url, workspace_id):
        (String, String, String, String, String, Option<String>, Option<String>, Option<String>, Option<String>, String) = {
        let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT r.method, r.url, r.headers_json, r.params_json, r.body_type,
                    r.body_json, r.body_raw, r.body_form_data, r.body_urlencoded,
                    c.workspace_id
             FROM requests r
             JOIN collections c ON r.collection_id = c.id
             WHERE r.id = ?"
        ).map_err(|e| e.to_string())?;
        stmt.query_row([&request_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
            ))
        }).map_err(|e| format!("Request not found: {}", e))?
    };

    // Interpolate URL
    let interpolated_url_base = interpolate_variables(db_path, &workspace_id, &raw_url);

    // Interpolate params and build full URL
    #[derive(serde::Deserialize)]
    struct SimpleParam { key: String, value: String, #[serde(default)] enabled: bool }

    fn url_encode(s: &str) -> String {
        let mut out = String::new();
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
                _ => out.push_str(&format!("%{:02X}", b)),
            }
        }
        out
    }

    let params_list: Vec<SimpleParam> = serde_json::from_str(&params_json).unwrap_or_default();
    let mut final_url = interpolated_url_base.clone();
    let active_params: Vec<String> = params_list.into_iter()
        .filter(|p| p.enabled && !p.key.trim().is_empty())
        .map(|p| format!(
            "{}={}",
            url_encode(&interpolate_variables(db_path, &workspace_id, &p.key)),
            url_encode(&interpolate_variables(db_path, &workspace_id, &p.value))
        ))
        .collect();
    if !active_params.is_empty() {
        let joiner = if final_url.contains('?') { "&" } else { "?" };
        final_url = format!("{}{}{}", final_url, joiner, active_params.join("&"));
    }
    if !final_url.starts_with("http://") && !final_url.starts_with("https://") {
        final_url = format!("https://{}", final_url);
    }

    // Interpolate headers
    #[derive(serde::Deserialize)]
    struct SimpleHeader { key: String, value: String, #[serde(default)] enabled: bool }

    let headers_list: Vec<SimpleHeader> = serde_json::from_str(&headers_json).unwrap_or_default();
    let interpolated_headers: Vec<(String, String)> = headers_list.into_iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(|h| (
            interpolate_variables(db_path, &workspace_id, &h.key),
            interpolate_variables(db_path, &workspace_id, &h.value),
        ))
        .collect();

    // Select and interpolate body
    let raw_body = match body_type.as_str() {
        "json" => b_json,
        "raw" => b_raw,
        "form-data" | "multipart" => b_form,
        "urlencoded" | "x-www-form-urlencoded" => b_url,
        _ => None,
    };
    let interpolated_body = raw_body.map(|b| interpolate_variables(db_path, &workspace_id, &b));

    // ---- Build Full Request text (proper HTTP/1.1 format) ----
    // Extract host and path from final_url
    let (host_str, path_str) = {
        let without_scheme = final_url
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        let slash_pos = without_scheme.find('/').unwrap_or(without_scheme.len());
        let host = &without_scheme[..slash_pos];
        let path = &without_scheme[slash_pos..];
        (host.to_string(), if path.is_empty() { "/".to_string() } else { path.to_string() })
    };

    let has_content_type = interpolated_headers.iter().any(|(k, _)| k.to_lowercase() == "content-type");

    // ---- Build Full Request (Full URL format) ----
    let mut full_url_request = format!("{} {} HTTP/1.1\r\n", method.to_uppercase(), final_url);
    for (k, v) in &interpolated_headers {
        full_url_request.push_str(&format!("{}: {}\r\n", k, v));
    }
    if !has_content_type {
        match body_type.as_str() {
            "json" => full_url_request.push_str("Content-Type: application/json\r\n"),
            "urlencoded" | "x-www-form-urlencoded" => full_url_request.push_str("Content-Type: application/x-www-form-urlencoded\r\n"),
            _ => {}
        }
    }
    full_url_request.push_str("\r\n");
    if let Some(ref body) = interpolated_body {
        full_url_request.push_str(body);
    }

    // ---- Build Full Request (Wire format with Host header) ----
    let mut full_request = format!("{} {} HTTP/1.1\r\n", method.to_uppercase(), path_str);
    full_request.push_str(&format!("Host: {}\r\n", host_str));
    for (k, v) in &interpolated_headers {
        full_request.push_str(&format!("{}: {}\r\n", k, v));
    }
    if !has_content_type {
        match body_type.as_str() {
            "json" => full_request.push_str("Content-Type: application/json\r\n"),
            "urlencoded" | "x-www-form-urlencoded" => full_request.push_str("Content-Type: application/x-www-form-urlencoded\r\n"),
            _ => {}
        }
    }
    full_request.push_str("\r\n");
    if let Some(ref body) = interpolated_body {
        full_request.push_str(body);
    }

    // ---- Build cURL command ----
    let mut curl_parts: Vec<String> = vec![
        format!("curl -X {}", method.to_uppercase()),
        format!("  '{}'", final_url),
    ];
    for (k, v) in &interpolated_headers {
        curl_parts.push(format!("  -H '{}: {}'", k, v));
    }
    if !has_content_type {
        match body_type.as_str() {
            "json" => curl_parts.push("  -H 'Content-Type: application/json'".to_string()),
            "urlencoded" | "x-www-form-urlencoded" => curl_parts.push("  -H 'Content-Type: application/x-www-form-urlencoded'".to_string()),
            _ => {}
        }
    }
    if let Some(ref body) = interpolated_body {
        let escaped = body.replace('\'', "'\\''");
        match body_type.as_str() {
            "urlencoded" | "x-www-form-urlencoded" => curl_parts.push(format!("  --data-urlencode '{}'", escaped)),
            _ => curl_parts.push(format!("  -d '{}'", escaped)),
        }
    }
    let curl_command = curl_parts.join(" \\\n");

    Ok(RequestPreview {
        method: method.to_uppercase(),
        url: final_url,
        host: host_str,
        path: path_str,
        headers: interpolated_headers,
        body: interpolated_body,
        body_type,
        full_url_request,
        full_request,
        curl_command,
    })
}
