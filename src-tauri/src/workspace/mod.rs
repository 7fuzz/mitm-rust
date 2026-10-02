pub mod import;
pub mod export;

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

/// Deleting the active environment hands "active" to the oldest remaining one, if any.
pub fn delete_workspace_environment_db(db_path: &PathBuf, id: &str) -> Result<(), String> {
    let mut conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let (workspace_id, was_active): (String, bool) = tx
        .query_row(
            "SELECT workspace_id, is_active FROM environments WHERE id = ?",
            params![id],
            |row| Ok((row.get(0)?, row.get::<_, i64>(1)? != 0)),
        )
        .map_err(|e| format!("Environment not found: {}", e))?;

    tx.execute("DELETE FROM environments WHERE id = ?", params![id])
        .map_err(|e| e.to_string())?;

    if was_active {
        let next: Option<String> = tx
            .query_row(
                "SELECT id FROM environments WHERE workspace_id = ? ORDER BY created_at_ms ASC LIMIT 1",
                params![workspace_id],
                |row| row.get(0),
            )
            .ok();
        if let Some(next_id) = &next {
            tx.execute("UPDATE environments SET is_active = 1 WHERE id = ?", params![next_id])
                .map_err(|e| e.to_string())?;
        }
        tx.execute(
            "UPDATE workspaces SET active_environment_id = ?, updated_at_ms = ? WHERE id = ?",
            params![next, now_ms(), workspace_id],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())
}

pub fn resolve_dynamic_variable(key: &str) -> Option<String> {
    let lower = key.trim().to_lowercase();
    let norm = lower.trim_start_matches('$');

    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    let millis = duration.as_millis();

    match norm {
        "timestamp" => Some(secs.to_string()),
        "timestampms" | "timestamp_ms" => Some(millis.to_string()),
        "isotimestamp" | "iso_timestamp" => {
            let offset_dt = time::OffsetDateTime::from_unix_timestamp(secs as i64)
                .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
            Some(
                offset_dt
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_else(|_| format!("{}Z", secs)),
            )
        }
        "currentdate" | "current_date" | "date" => {
            let offset_dt = time::OffsetDateTime::from_unix_timestamp(secs as i64)
                .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
            Some(format!(
                "{:04}-{:02}-{:02}",
                offset_dt.year(),
                offset_dt.month() as u8,
                offset_dt.day()
            ))
        }
        "currenttime" | "current_time" | "time" => {
            let offset_dt = time::OffsetDateTime::from_unix_timestamp(secs as i64)
                .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
            Some(format!(
                "{:02}:{:02}:{:02}",
                offset_dt.hour(),
                offset_dt.minute(),
                offset_dt.second()
            ))
        }
        "guid" | "uuid" | "uuidv4" | "uuid_v4" | "randomuuid" | "random_uuid" | "randomguid" | "random_guid" | "randomuuidv4" | "random_uuid_v4" => {
            Some(Uuid::new_v4().to_string())
        }
        "uuidv7" | "uuid_v7" | "guidv7" | "guid_v7" | "randomuuidv7" | "random_uuid_v7" => {
            Some(Uuid::now_v7().to_string())
        }
        "randomint" | "random_int" | "randominteger" | "random_integer" => {
            let u = Uuid::new_v4().as_u128();
            let val = (u % 1000) + 1;
            Some(val.to_string())
        }
        "randomdigit" | "random_digit" => {
            let u = Uuid::new_v4().as_u128();
            let val = u % 10;
            Some(val.to_string())
        }
        "randomalphanumeric" | "random_alphanumeric" => {
            let u = Uuid::new_v4().simple().to_string();
            Some(u[..8].to_string())
        }
        "randomhex" | "random_hex" => {
            let u = Uuid::new_v4().simple().to_string();
            Some(u[..16].to_string())
        }
        "randomprice" | "random_price" => {
            let u = Uuid::new_v4().as_u128();
            let whole = (u % 100) + 1;
            let cents = (u / 100) % 100;
            Some(format!("{}.{:02}", whole, cents))
        }
        "randomport" | "random_port" => {
            let u = Uuid::new_v4().as_u128();
            let port = 1024 + (u % 64000);
            Some(port.to_string())
        }
        "randomfirstname" | "random_first_name" => {
            let names = ["Alex", "Jordan", "Taylor", "Morgan", "Casey", "Riley", "Sam", "Chris", "Jamie", "Pat"];
            let u = Uuid::new_v4().as_u128() as usize;
            Some(names[u % names.len()].to_string())
        }
        "randomlastname" | "random_last_name" => {
            let names = ["Smith", "Johnson", "Williams", "Brown", "Jones", "Miller", "Davis", "Wilson", "Moore", "Taylor"];
            let u = Uuid::new_v4().as_u128() as usize;
            Some(names[u % names.len()].to_string())
        }
        "randomfullname" | "random_full_name" | "randomname" | "random_name" => {
            let firsts = ["Alex", "Jordan", "Taylor", "Morgan", "Casey", "Riley", "Sam", "Chris", "Jamie", "Pat"];
            let lasts = ["Smith", "Johnson", "Williams", "Brown", "Jones", "Miller", "Davis", "Wilson", "Moore", "Taylor"];
            let u = Uuid::new_v4().as_u128() as usize;
            Some(format!("{} {}", firsts[u % firsts.len()], lasts[(u / 10) % lasts.len()]))
        }
        "randomemail" | "random_email" => {
            let u = Uuid::new_v4().simple().to_string();
            Some(format!("user_{}@example.com", &u[..8]))
        }
        "randomphonenumber" | "random_phone_number" | "randomphone" | "random_phone" => {
            let u = Uuid::new_v4().as_u128();
            let num = (u % 900000000) + 100000000;
            Some(format!("+6281{}", num))
        }
        "randomusername" | "random_user_name" => {
            let u = Uuid::new_v4().simple().to_string();
            Some(format!("user_{}", &u[..6]))
        }
        "randompassword" | "random_password" => {
            let u = Uuid::new_v4().simple().to_string();
            Some(format!("Pass!{}", &u[..10]))
        }
        "randomcity" | "random_city" => {
            let cities = ["Jakarta", "Bandung", "Surabaya", "Medan", "Semarang", "Yogyakarta", "Denpasar", "Makassar"];
            let u = Uuid::new_v4().as_u128() as usize;
            Some(cities[u % cities.len()].to_string())
        }
        "randomcountry" | "random_country" => {
            let countries = ["Indonesia", "Singapore", "Malaysia", "Japan", "United States", "Germany", "Australia"];
            let u = Uuid::new_v4().as_u128() as usize;
            Some(countries[u % countries.len()].to_string())
        }
        "randomcountrycode" | "random_country_code" => {
            let codes = ["ID", "SG", "MY", "JP", "US", "DE", "AU"];
            let u = Uuid::new_v4().as_u128() as usize;
            Some(codes[u % codes.len()].to_string())
        }
        "randomipv4" | "random_ipv4" | "randomip" | "random_ip" => {
            let u = Uuid::new_v4().as_u128();
            Some(format!("{}.{}.{}.{}", (u >> 24) & 0xFF, (u >> 16) & 0xFF, (u >> 8) & 0xFF, u & 0xFF))
        }
        "randomboolean" | "random_boolean" | "randombool" | "random_bool" => {
            let u = Uuid::new_v4().as_u128();
            if u % 2 == 0 {
                Some("true".to_string())
            } else {
                Some("false".to_string())
            }
        }
        _ => None,
    }
}

pub fn interpolate_variables_with_env(env: &Environment, text: &str) -> String {
    if !text.contains("{{") || !text.contains("}}") {
        return text.to_string();
    }

    let mut var_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut lower_var_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for var in &env.variables {
        if var.enabled && !var.key.trim().is_empty() {
            let k = var.key.trim().to_string();
            lower_var_map.insert(k.to_lowercase(), var.value.clone());
            var_map.insert(k, var.value.clone());
        }
    }

    let re = match regex::Regex::new(r"\{\{\s*([^\r\n{}]+?)\s*\}\}") {
        Ok(r) => r,
        Err(_) => return text.to_string(),
    };

    let mut result = text.to_string();
    const MAX_DEPTH: usize = 15;

    for _ in 0..MAX_DEPTH {
        if !result.contains("{{") || !result.contains("}}") {
            break;
        }

        let mut changed = false;
        let next_result = re
            .replace_all(&result, |caps: &regex::Captures| {
                let key = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
                // 1. Try exact user variable match
                if let Some(val) = var_map.get(key) {
                    changed = true;
                    val.to_string()
                // 2. Try case-insensitive user variable match
                } else if let Some(val) = lower_var_map.get(&key.to_lowercase()) {
                    changed = true;
                    val.to_string()
                // 3. Try dynamic built-in variable match (e.g. {{$timestamp}}, {{$guid}}, {{$randomUUID}})
                } else if let Some(val) = resolve_dynamic_variable(key) {
                    changed = true;
                    val
                } else {
                    caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                }
            })
            .to_string();

        if !changed || next_result == result {
            break;
        }
        result = next_result;
    }

    result
}

pub fn interpolate_dynamic_variables(text: &str) -> String {
    if !text.contains("{{") || !text.contains("}}") {
        return text.to_string();
    }

    let re = match regex::Regex::new(r"\{\{\s*([^\r\n{}]+?)\s*\}\}") {
        Ok(r) => r,
        Err(_) => return text.to_string(),
    };

    re.replace_all(text, |caps: &regex::Captures| {
        let key = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        if let Some(val) = resolve_dynamic_variable(key) {
            val
        } else {
            caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
        }
    })
    .to_string()
}

pub fn interpolate_variables(db_path: &PathBuf, workspace_id: &str, text: &str) -> String {
    if !text.contains("{{") || !text.contains("}}") {
        return text.to_string();
    }

    let envs = get_workspace_environments_db(db_path, workspace_id).unwrap_or_default();
    let active_env = envs.into_iter().find(|e| e.is_active);

    if let Some(env) = active_env {
        interpolate_variables_with_env(&env, text)
    } else {
        interpolate_dynamic_variables(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_timestamp_and_guid() {
        let text = "postman-{{$timestamp}}";
        let result = interpolate_dynamic_variables(text);
        assert!(result.starts_with("postman-"));
        let ts_str = result.trim_start_matches("postman-");
        assert!(ts_str.parse::<u64>().is_ok());

        let guid_text = "UUID: {{$guid}}";
        let guid_result = interpolate_dynamic_variables(guid_text);
        assert!(guid_result.starts_with("UUID: "));
        let uuid_str = guid_result.trim_start_matches("UUID: ");
        assert_eq!(uuid_str.len(), 36);
    }

    #[test]
    fn test_uuidv4_and_uuidv7() {
        let v4 = interpolate_dynamic_variables("{{$uuid}}");
        assert_eq!(v4.len(), 36);
        let parsed_v4 = Uuid::parse_str(&v4).expect("valid uuid");
        assert_eq!(parsed_v4.get_version_num(), 4);

        let v7 = interpolate_dynamic_variables("{{$uuidv7}}");
        assert_eq!(v7.len(), 36);
        let parsed_v7 = Uuid::parse_str(&v7).expect("valid uuid v7");
        assert_eq!(parsed_v7.get_version_num(), 7);
    }

    #[test]
    fn test_nested_double_interpolation() {
        let env = Environment {
            id: "env-1".to_string(),
            workspace_id: "ws-1".to_string(),
            name: "Test Env".to_string(),
            is_active: true,
            variables: vec![
                EnvironmentVariable {
                    key: "var_a".to_string(),
                    value: "abc".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
                EnvironmentVariable {
                    key: "var_b".to_string(),
                    value: "{{var_a}}def".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
            ],
            created_at_ms: 0,
            updated_at_ms: 0,
        };

        let result = interpolate_variables_with_env(&env, "https://api.com/{{var_b}}");
        assert_eq!(result, "https://api.com/abcdef");

        let result_spaces = interpolate_variables_with_env(&env, "https://api.com/{{ var_b }}");
        assert_eq!(result_spaces, "https://api.com/abcdef");

        let result_dynamic = interpolate_variables_with_env(&env, "X-Key: {{var_a}}-{{$timestamp}}");
        assert!(result_dynamic.starts_with("X-Key: abc-"));
    }

    #[test]
    fn test_deeply_nested_interpolation() {
        let env = Environment {
            id: "env-2".to_string(),
            workspace_id: "ws-1".to_string(),
            name: "Deep Env".to_string(),
            is_active: true,
            variables: vec![
                EnvironmentVariable {
                    key: "HOST".to_string(),
                    value: "example.com".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
                EnvironmentVariable {
                    key: "BASE_URL".to_string(),
                    value: "https://{{HOST}}/api".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
                EnvironmentVariable {
                    key: "USERS_ENDPOINT".to_string(),
                    value: "{{BASE_URL}}/v1/users".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
            ],
            created_at_ms: 0,
            updated_at_ms: 0,
        };

        let result = interpolate_variables_with_env(&env, "{{USERS_ENDPOINT}}?page=1");
        assert_eq!(result, "https://example.com/api/v1/users?page=1");
    }

    #[test]
    fn test_circular_reference_safety() {
        let env = Environment {
            id: "env-3".to_string(),
            workspace_id: "ws-1".to_string(),
            name: "Circular Env".to_string(),
            is_active: true,
            variables: vec![
                EnvironmentVariable {
                    key: "a".to_string(),
                    value: "{{b}}".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
                EnvironmentVariable {
                    key: "b".to_string(),
                    value: "{{a}}".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
            ],
            created_at_ms: 0,
            updated_at_ms: 0,
        };

        // Should not loop infinitely or panic
        let _ = interpolate_variables_with_env(&env, "{{a}}");
    }
}

