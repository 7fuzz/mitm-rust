use std::collections::HashMap;
use std::path::PathBuf;
use rusqlite::{params, Connection};
use serde_json::json;

use crate::workspace::get_workspace_environments_db;
use crate::collections::{get_collections_db, CollectionTreeItem, RequestItem};

/// Formats a request URL to strictly adhere to the MITM project import specification:
/// Every relative endpoint MUST explicitly start with `{{<identity>}}/` (e.g. `{{url}}/api/...`).
pub fn format_endpoint(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return "{{url}}/".to_string();
    }

    if trimmed.starts_with("{{") || trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else if trimmed.starts_with('/') {
        format!("{{{{url}}}}{}", trimmed)
    } else {
        format!("{{{{url}}}}/{}", trimmed)
    }
}

/// Request names by id, so a pre-request link can be exported as the name the importer resolves.
fn request_names(nodes: &[CollectionTreeItem], names: &mut HashMap<String, String>) {
    for node in nodes {
        for req in &node.requests {
            names.insert(req.id.clone(), req.name.clone());
        }
        request_names(&node.children, names);
    }
}

fn serialize_request_item(req: &RequestItem, names: &HashMap<String, String>) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    obj.insert("name".to_string(), json!(req.name));
    obj.insert("method".to_string(), json!(req.method.to_uppercase()));
    obj.insert("endpoint".to_string(), json!(format_endpoint(&req.url)));

    if let Some(ref desc) = req.description {
        if !desc.trim().is_empty() {
            obj.insert("description".to_string(), json!(desc));
        }
    }

    // Active headers dictionary
    let mut header_map = serde_json::Map::new();
    for h in &req.headers {
        if h.enabled && !h.key.trim().is_empty() {
            header_map.insert(h.key.clone(), json!(h.value));
        }
    }
    obj.insert("header".to_string(), serde_json::Value::Object(header_map));

    // Body mode & content
    let body_mode = match req.body_type.to_lowercase().as_str() {
        "form-data" | "multipart" => "multipart",
        "urlencoded" => "urlencoded",
        "json" => "json",
        "raw" => "raw",
        _ => "none",
    };
    obj.insert("body_mode".to_string(), json!(body_mode));

    let body_content = match body_mode {
        "json" => req.body_json.clone().or_else(|| req.body_raw.clone()).unwrap_or_default(),
        "urlencoded" => req.body_urlencoded.clone().or_else(|| req.body_raw.clone()).unwrap_or_default(),
        "multipart" => req.body_form_data.clone().or_else(|| req.body_raw.clone()).unwrap_or_default(),
        "raw" => req.body_raw.clone().unwrap_or_default(),
        _ => req.body_raw.clone().unwrap_or_default(),
    };
    obj.insert("body".to_string(), json!(body_content));

    if let Some(ref bj) = req.body_json {
        if !bj.is_empty() {
            obj.insert("body_json".to_string(), json!(bj));
        }
    }
    if let Some(ref bu) = req.body_urlencoded {
        if !bu.is_empty() {
            obj.insert("body_urlencoded".to_string(), json!(bu));
        }
    }
    if let Some(ref bm) = req.body_form_data {
        if !bm.is_empty() {
            obj.insert("body_multipart".to_string(), json!(bm));
        }
    }

    // Query parameters
    if !req.params.is_empty() {
        let mut param_map = serde_json::Map::new();
        for p in &req.params {
            if p.enabled && !p.key.trim().is_empty() {
                param_map.insert(p.key.clone(), json!(p.value));
            }
        }
        obj.insert("params".to_string(), serde_json::Value::Object(param_map));

        let url_params_data: Vec<serde_json::Value> = req.params.iter().map(|p| {
            json!({
                "id": p.id,
                "k": p.key,
                "v": p.value,
                "enabled": p.enabled,
            })
        }).collect();
        if let Ok(up_str) = serde_json::to_string(&url_params_data) {
            obj.insert("url_params".to_string(), json!(up_str));
        }
    }

    // Auto-extraction rules
    if !req.extract_rules.is_empty() {
        let extract_list: Vec<serde_json::Value> = req.extract_rules.iter().map(|r| {
            json!({
                "type": r.r#type,
                "targetVariable": r.target_variable,
                "expression": r.expression,
                "enabled": r.enabled,
            })
        }).collect();
        obj.insert("extract".to_string(), serde_json::Value::Array(extract_list));
    }

    for (key, ids) in [("pre_requests", &req.pre_requests), ("post_requests", &req.post_requests)] {
        let step_names: Vec<&String> = ids.iter().filter_map(|id| names.get(id)).collect();
        if !step_names.is_empty() {
            obj.insert(key.to_string(), json!(step_names));
        }
    }

    serde_json::Value::Object(obj)
}

fn serialize_collection_node(node: &CollectionTreeItem, names: &HashMap<String, String>) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    obj.insert("name".to_string(), json!(node.name));

    if let Some(ref desc) = node.description {
        if !desc.trim().is_empty() {
            obj.insert("description".to_string(), json!(desc));
        }
    }

    let targets: Vec<serde_json::Value> = node.requests.iter().map(|r| serialize_request_item(r, names)).collect();
    obj.insert("target".to_string(), serde_json::Value::Array(targets));

    let folders: Vec<serde_json::Value> = node.children.iter().map(|c| serialize_collection_node(c, names)).collect();
    obj.insert("folders".to_string(), serde_json::Value::Array(folders));

    serde_json::Value::Object(obj)
}

pub fn export_workspace_json_db(db_path: &PathBuf, workspace_id: &str) -> Result<String, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // 1. Fetch Workspace metadata
    let (ws_name, ws_desc): (String, Option<String>) = conn.query_row(
        "SELECT name, description FROM workspaces WHERE id = ?",
        params![workspace_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|e| format!("Workspace with ID '{}' not found: {}", workspace_id, e))?;

    // 2. Fetch Environments & Variables
    let envs = get_workspace_environments_db(db_path, workspace_id)?;

    let mut all_environments = Vec::new();
    let mut all_variables = Vec::new();
    let mut detected_url_var = None;

    for env in &envs {
        all_environments.push(json!({
            "id": env.id,
            "name": env.name
        }));

        for var in &env.variables {
            let key_lower = var.key.trim().to_lowercase();
            if key_lower == "url" || key_lower == "base_url" {
                detected_url_var = Some("{{url}}");
            } else if key_lower == "host" && detected_url_var.is_none() {
                detected_url_var = Some("https://{{host}}");
            }

            let mut variant_values = Vec::new();
            for v in &var.variants {
                variant_values.push(json!({
                    "name": v.name,
                    "value": v.value
                }));
            }

            if variant_values.is_empty() {
                variant_values.push(json!({
                    "name": "(auto)",
                    "value": var.value
                }));
            }

            all_variables.push(json!({
                "environmentId": env.id,
                "name": var.key,
                "activeIndex": var.active_index,
                "values": variant_values
            }));
        }
    }

    // Fallback base URL template
    let root_url = detected_url_var.unwrap_or("https://{{host}}");

    // 3. Fetch Collections & Requests Tree
    let collections_tree = get_collections_db(db_path, workspace_id)?;
    let mut names = HashMap::new();
    request_names(&collections_tree, &mut names);
    let test_cases: Vec<serde_json::Value> = collections_tree
        .iter()
        .map(|c| serialize_collection_node(c, &names))
        .collect();

    // 4. Construct Root Project Document
    let mut root = serde_json::Map::new();
    root.insert("name".to_string(), json!(ws_name));
    if let Some(ref desc) = ws_desc {
        if !desc.trim().is_empty() {
            root.insert("description".to_string(), json!(desc));
        }
    }
    root.insert("url".to_string(), json!(root_url));
    root.insert("all_environments".to_string(), json!(all_environments));
    root.insert("all_variables".to_string(), json!(all_variables));
    root.insert("test_cases".to_string(), json!(test_cases));

    serde_json::to_string_pretty(&root)
        .map_err(|e| format!("Failed to serialize workspace to JSON: {}", e))
}

pub fn export_workspace_file_db(
    db_path: &PathBuf,
    workspace_id: &str,
    destination_path: &str,
) -> Result<(), String> {
    let json_content = export_workspace_json_db(db_path, workspace_id)?;
    let target_path = std::path::Path::new(destination_path);

    if let Some(parent) = target_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create destination directory: {}", e))?;
        }
    }

    std::fs::write(target_path, json_content)
        .map_err(|e| format!("Failed to write workspace JSON to '{}': {}", destination_path, e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_endpoint_compliance() {
        // Relative endpoints MUST be prefixed with {{url}}/
        assert_eq!(format_endpoint("/api/v1/users"), "{{url}}/api/v1/users");
        assert_eq!(format_endpoint("api/v1/users"), "{{url}}/api/v1/users");
        assert_eq!(format_endpoint(""), "{{url}}/");

        // Endpoints already template-prefixed or absolute must be preserved
        assert_eq!(format_endpoint("{{host}}/api/v1/users"), "{{host}}/api/v1/users");
        assert_eq!(format_endpoint("{{app-dev}}/api/users"), "{{app-dev}}/api/users");
        assert_eq!(format_endpoint("https://example.com/test"), "https://example.com/test");
        assert_eq!(format_endpoint("http://localhost:8080/metrics"), "http://localhost:8080/metrics");
    }

    #[test]
    fn test_export_workspace_round_trip() {
        let temp_dir = std::env::temp_dir().join(format!("mitm_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch("
            CREATE TABLE workspaces (id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT, active_environment_id TEXT, created_at_ms INTEGER, updated_at_ms INTEGER);
            CREATE TABLE environments (id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, name TEXT NOT NULL, is_active INTEGER NOT NULL, variables_json TEXT NOT NULL DEFAULT '[]', created_at_ms INTEGER, updated_at_ms INTEGER);
            CREATE TABLE collections (id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, parent_id TEXT, name TEXT NOT NULL, description TEXT, order_index INTEGER DEFAULT 0, created_at_ms INTEGER, updated_at_ms INTEGER);
            CREATE TABLE requests (id TEXT PRIMARY KEY, collection_id TEXT NOT NULL, name TEXT NOT NULL, method TEXT NOT NULL, url TEXT NOT NULL, headers_json TEXT NOT NULL, params_json TEXT NOT NULL, body_type TEXT NOT NULL, body_json TEXT, body_raw TEXT, body_form_data TEXT, body_urlencoded TEXT, extract_rules_json TEXT NOT NULL, description TEXT, order_index INTEGER DEFAULT 0, created_at_ms INTEGER, updated_at_ms INTEGER);
        ").unwrap();
        conn.execute_batch(include_str!("../../migrations/20261005_0019_request_steps.sql")).unwrap();

        // 1. Insert Workspace
        conn.execute(
            "INSERT INTO workspaces VALUES ('ws-1', 'Test Suite', 'Workspace description', 'env-1', 1000, 1000)",
            [],
        ).unwrap();

        // 2. Insert Environment
        let vars_json = serde_json::json!([
            {
                "key": "host",
                "value": "api.example.com",
                "enabled": true,
                "type": "default",
                "activeIndex": 0,
                "variants": [
                    { "name": "(auto)", "value": "api.example.com" },
                    { "name": "Staging", "value": "staging.example.com" }
                ]
            }
        ]).to_string();
        conn.execute(
            "INSERT INTO environments VALUES ('env-1', 'ws-1', 'Staging Env', 1, ?, 1000, 1000)",
            params![vars_json],
        ).unwrap();

        // 3. Insert Collection and Nested Subfolder
        conn.execute(
            "INSERT INTO collections VALUES ('col-1', 'ws-1', NULL, 'Auth Group', 'Group description', 0, 1000, 1000)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO collections VALUES ('col-sub', 'ws-1', 'col-1', 'OAuth Subfolder', 'Subfolder description', 0, 1001, 1001)",
            [],
        ).unwrap();

        // 4. Insert Request in Root Collection
        let headers_json = serde_json::json!([
            { "id": "h1", "key": "Content-Type", "value": "application/json", "enabled": true }
        ]).to_string();
        let params_json = serde_json::json!([
            { "id": "p1", "key": "debug", "value": "1", "enabled": true }
        ]).to_string();
        let extract_json = serde_json::json!([
            { "id": "e1", "type": "json", "targetVariable": "token", "expression": "data.token", "enabled": true }
        ]).to_string();

        conn.execute(
            "INSERT INTO requests VALUES ('req-1', 'col-1', 'Login', 'POST', '/auth/login', ?, ?, 'json', '{\"user\":\"admin\"}', '{\"user\":\"admin\"}', NULL, NULL, ?, 'Login request', 0, 1000, 1000)",
            params![headers_json, params_json, extract_json],
        ).unwrap();

        drop(conn);

        // Run Export
        let exported_json = export_workspace_json_db(&db_path, "ws-1").expect("Export must succeed");
        let parsed: serde_json::Value = serde_json::from_str(&exported_json).expect("Export must be valid JSON");

        assert_eq!(parsed["name"], "Test Suite");
        assert_eq!(parsed["description"], "Workspace description");
        assert_eq!(parsed["all_environments"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["all_variables"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["all_variables"][0]["name"], "host");
        assert_eq!(parsed["all_variables"][0]["values"].as_array().unwrap().len(), 2);

        // Verify test_cases hierarchy
        let test_cases = parsed["test_cases"].as_array().unwrap();
        assert_eq!(test_cases.len(), 1);
        assert_eq!(test_cases[0]["name"], "Auth Group");
        assert_eq!(test_cases[0]["target"].as_array().unwrap().len(), 1);
        assert_eq!(test_cases[0]["folders"].as_array().unwrap().len(), 1);
        assert_eq!(test_cases[0]["folders"][0]["name"], "OAuth Subfolder");

        // Verify request target endpoint prefix rule
        let target_req = &test_cases[0]["target"][0];
        assert_eq!(target_req["name"], "Login");
        assert_eq!(target_req["method"], "POST");
        assert_eq!(target_req["endpoint"], "{{url}}/auth/login");
        assert_eq!(target_req["header"]["Content-Type"], "application/json");
        assert_eq!(target_req["body_mode"], "json");
        assert_eq!(target_req["body"], "{\"user\":\"admin\"}");
        assert_eq!(target_req["extract"].as_array().unwrap().len(), 1);

        // Test round-trip import into a new workspace
        let import_summary = crate::workspace::import::import_workspace_json_db(
            &db_path,
            &exported_json,
            None,
            Some("Re-imported Workspace"),
        ).expect("Import of exported JSON must succeed");

        assert_eq!(import_summary.workspace_name, "Re-imported Workspace");
        assert_eq!(import_summary.environments_imported, 1);
        assert_eq!(import_summary.collections_imported, 2);
        assert_eq!(import_summary.requests_imported, 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn request_steps_survive_import_and_export() {
        let db_path = std::env::temp_dir().join(format!("mitm_request_steps_{}.db", uuid::Uuid::new_v4()));
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            for m in crate::db::migrations::MIGRATIONS {
                for statement in m.sql.split(';').map(str::trim).filter(|s| !s.is_empty()) {
                    let _ = conn.execute(statement, []);
                }
            }
        }
        let project = r#"{
            "name": "CSRF",
            "test_cases": [
                { "name": "Other", "target": [ { "name": "Get CSRF Token", "endpoint": "https://a/other" } ] },
                { "name": "Account", "target": [
                    { "name": "Login", "endpoint": "https://a/login" },
                    { "name": "Get CSRF Token", "endpoint": "https://a/form" },
                    { "name": "Logout", "endpoint": "https://a/logout" },
                    { "name": "Change Email", "method": "POST", "endpoint": "https://a/email",
                      "pre_requests": ["Login", "Get CSRF Token", "Missing"], "post_requests": ["Logout"] }
                ] }
            ]
        }"#;

        let summary = crate::workspace::import::import_workspace_json_db(&db_path, project, None, None).unwrap();
        let tree = get_collections_db(&db_path, &summary.workspace_id).unwrap();
        let account = tree.iter().find(|c| c.name == "Account").unwrap();
        let id = |name: &str| account.requests.iter().find(|r| r.name == name).unwrap().id.clone();
        let change_email = account.requests.iter().find(|r| r.name == "Change Email").unwrap();
        let exported: serde_json::Value =
            serde_json::from_str(&export_workspace_json_db(&db_path, &summary.workspace_id).unwrap()).unwrap();
        let _ = std::fs::remove_file(&db_path);

        assert_eq!(change_email.pre_requests, [id("Login"), id("Get CSRF Token")]);
        assert_eq!(change_email.post_requests, [id("Logout")]);
        let exported_target = &exported["test_cases"][1]["target"][3];
        assert_eq!(exported_target["pre_requests"], serde_json::json!(["Login", "Get CSRF Token"]));
        assert_eq!(exported_target["post_requests"], serde_json::json!(["Logout"]));
    }
}
