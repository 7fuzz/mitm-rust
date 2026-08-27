pub mod import;

use std::path::PathBuf;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableVariant {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentVariable {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_type")]
    pub r#type: String, // 'default' | 'secret'
    #[serde(default)]
    pub active_index: usize,
    #[serde(default)]
    pub variants: Vec<VariableVariant>,
}

fn default_true() -> bool {
    true
}
fn default_type() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub is_active: bool,
    pub variables: Vec<EnvironmentVariable>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub active_environment_id: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn ensure_schema_columns(conn: &Connection) {
    let _ = conn.execute("ALTER TABLE environments ADD COLUMN workspace_id TEXT NOT NULL DEFAULT ''", []);
    let _ = conn.execute("ALTER TABLE environments ADD COLUMN variables_json TEXT NOT NULL DEFAULT '[]'", []);
    let _ = conn.execute("ALTER TABLE collections ADD COLUMN workspace_id TEXT NOT NULL DEFAULT ''", []);
}

pub fn get_workspaces_db(db_path: &PathBuf) -> Result<Vec<Workspace>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    ensure_schema_columns(&conn);
    let mut stmt = conn
        .prepare("SELECT id, name, description, active_environment_id, created_at_ms, updated_at_ms FROM workspaces ORDER BY updated_at_ms DESC")
        .map_err(|e| e.to_string())?;

    let workspaces = stmt
        .query_map([], |row| {
            Ok(Workspace {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                active_environment_id: row.get(3)?,
                created_at_ms: row.get(4)?,
                updated_at_ms: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(workspaces)
}

pub fn create_workspace_db(
    db_path: &PathBuf,
    name: String,
    description: Option<String>,
) -> Result<Workspace, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();
    let id = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO workspaces (id, name, description, active_environment_id, created_at_ms, updated_at_ms)
         VALUES (?, ?, ?, NULL, ?, ?)",
        params![id, name, description, ts, ts],
    )
    .map_err(|e| e.to_string())?;

    Ok(Workspace {
        id,
        name,
        description,
        active_environment_id: None,
        created_at_ms: ts,
        updated_at_ms: ts,
    })
}

pub fn update_workspace_db(db_path: &PathBuf, workspace: Workspace) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();

    conn.execute(
        "UPDATE workspaces SET name = ?, description = ?, active_environment_id = ?, updated_at_ms = ? WHERE id = ?",
        params![workspace.name, workspace.description, workspace.active_environment_id, ts, workspace.id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn delete_workspace_db(db_path: &PathBuf, id: &str) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let _ = conn.execute("PRAGMA foreign_keys = ON;", []);

    // 1. Delete associated environments
    let _ = conn.execute("DELETE FROM environments WHERE workspace_id = ?", params![id]);

    // 2. Find and delete requests & request execution histories inside collections of this workspace
    let mut stmt = conn.prepare("SELECT id FROM collections WHERE workspace_id = ?").map_err(|e| e.to_string())?;
    let col_ids: Vec<String> = stmt
        .query_map([id], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    for col_id in col_ids {
        if let Ok(mut req_stmt) = conn.prepare("SELECT id FROM requests WHERE collection_id = ?") {
            let req_ids: Vec<String> = req_stmt
                .query_map([&col_id], |row| row.get(0))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();

            for req_id in req_ids {
                let _ = conn.execute("DELETE FROM request_histories WHERE request_id = ?", params![&req_id]);
            }
        }
        let _ = conn.execute("DELETE FROM requests WHERE collection_id = ?", params![&col_id]);
    }

    // 3. Delete collections and workspace record
    let _ = conn.execute("DELETE FROM collections WHERE workspace_id = ?", params![id]);
    conn.execute("DELETE FROM workspaces WHERE id = ?", params![id]).map_err(|e| e.to_string())?;

    Ok(())
}

pub fn set_active_workspace_db(db_path: &PathBuf, id: &str) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();

    conn.execute(
        "UPDATE workspaces SET updated_at_ms = ? WHERE id = ?",
        params![ts, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn get_workspace_environments_db(
    db_path: &PathBuf,
    workspace_id: &str,
) -> Result<Vec<Environment>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    ensure_schema_columns(&conn);
    let mut stmt = conn
        .prepare("SELECT id, workspace_id, name, is_active, variables_json, created_at_ms, updated_at_ms FROM environments WHERE workspace_id = ? ORDER BY created_at_ms ASC")
        .map_err(|e| e.to_string())?;

    let envs = stmt
        .query_map([workspace_id], |row| {
            let vars_json: String = row.get(4)?;
            let variables: Vec<EnvironmentVariable> = serde_json::from_str(&vars_json).unwrap_or_default();
            let is_active_int: i32 = row.get(3)?;

            Ok(Environment {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                is_active: is_active_int == 1,
                variables,
                created_at_ms: row.get(5)?,
                updated_at_ms: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(envs)
}

pub fn save_workspace_environment_db(
    db_path: &PathBuf,
    env: Environment,
) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();
    let vars_json = serde_json::to_string(&env.variables).unwrap_or_else(|_| "[]".to_string());
    let is_active_int = if env.is_active { 1 } else { 0 };

    if env.is_active {
        // Deactivate all other environments in this workspace
        let _ = conn.execute(
            "UPDATE environments SET is_active = 0 WHERE workspace_id = ?",
            params![env.workspace_id],
        );
        // Update active_environment_id on workspace
        let _ = conn.execute(
            "UPDATE workspaces SET active_environment_id = ?, updated_at_ms = ? WHERE id = ?",
            params![env.id, ts, env.workspace_id],
        );
    }

    conn.execute(
        "INSERT OR REPLACE INTO environments (id, workspace_id, name, is_active, variables_json, created_at_ms, updated_at_ms)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![env.id, env.workspace_id, env.name, is_active_int, vars_json, env.created_at_ms, ts],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn interpolate_variables(db_path: &PathBuf, workspace_id: &str, text: &str) -> String {
    if !text.contains("{{") || !text.contains("}}") {
        return text.to_string();
    }

    let envs = get_workspace_environments_db(db_path, workspace_id).unwrap_or_default();
    let active_env = envs.into_iter().find(|e| e.is_active);

    if let Some(env) = active_env {
        let mut result = text.to_string();
        for var in env.variables {
            if var.enabled && !var.key.trim().is_empty() {
                let placeholder = format!("{{{{{}}}}}", var.key.trim());
                result = result.replace(&placeholder, &var.value);
            }
        }
        result
    } else {
        text.to_string()
    }
}
