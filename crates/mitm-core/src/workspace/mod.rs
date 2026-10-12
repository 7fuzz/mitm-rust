pub mod export;
pub mod file;
pub mod import;

use std::path::{Path, PathBuf};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::repeater::REPEATER_WORKSPACE_ID;

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

impl EnvironmentVariable {
    pub fn current_value(&self) -> &str {
        if !self.variants.is_empty() && self.active_index < self.variants.len() {
            &self.variants[self.active_index].value
        } else {
            &self.value
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unresolved {
    pub name: String,
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
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    ensure_schema_columns(&conn);
    let mut stmt = conn
        .prepare("SELECT id, name, description, active_environment_id, created_at_ms, updated_at_ms FROM workspaces WHERE id != ? ORDER BY updated_at_ms DESC")
        .map_err(|e| e.to_string())?;

    // The Repeater workspace belongs to the Repeater view, not the workspace switcher
    let workspaces = stmt
        .query_map([REPEATER_WORKSPACE_ID], |row| {
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
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
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
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();

    if workspace.id == REPEATER_WORKSPACE_ID {
        conn.execute(
            "UPDATE workspaces SET active_environment_id = ?, updated_at_ms = ? WHERE id = ?",
            params![workspace.active_environment_id, ts, workspace.id],
        )
        .map_err(|e| e.to_string())?;
        return Ok(());
    }

    conn.execute(
        "UPDATE workspaces SET name = ?, description = ?, active_environment_id = ?, updated_at_ms = ? WHERE id = ?",
        params![workspace.name, workspace.description, workspace.active_environment_id, ts, workspace.id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn delete_workspace_db(db_path: &PathBuf, id: &str) -> Result<(), String> {
    if id == REPEATER_WORKSPACE_ID {
        return Err("The Repeater workspace can't be deleted.".to_string());
    }
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let _ = conn.execute("DELETE FROM environments WHERE workspace_id = ?", params![id]);
    let _ = conn.execute("DELETE FROM credentials WHERE workspace_id = ?", params![id]);

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

    let _ = conn.execute("DELETE FROM collections WHERE workspace_id = ?", params![id]);
    conn.execute("DELETE FROM workspaces WHERE id = ?", params![id]).map_err(|e| e.to_string())?;

    Ok(())
}

pub fn get_workspace_db(db_path: &PathBuf, id: &str) -> Result<Option<Workspace>, String> {
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    ensure_schema_columns(&conn);
    let mut stmt = conn
        .prepare("SELECT id, name, description, active_environment_id, created_at_ms, updated_at_ms FROM workspaces WHERE id = ?")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt
        .query_map([id], |row| {
            Ok(Workspace {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                active_environment_id: row.get(3)?,
                created_at_ms: row.get(4)?,
                updated_at_ms: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    if let Some(r) = rows.next() {
        r.map(Some).map_err(|e| e.to_string())
    } else {
        Ok(None)
    }
}

pub fn get_active_workspace_db(db_path: &PathBuf) -> Result<Option<Workspace>, String> {
    if let Some(active_id) = crate::db::get_preference(db_path, "active_workspace_id") {
        if let Ok(Some(ws)) = get_workspace_db(db_path, &active_id) {
            return Ok(Some(ws));
        }
    }
    let all = get_workspaces_db(db_path)?;
    if let Some(first) = all.into_iter().next() {
        let _ = crate::db::set_preference(db_path, "active_workspace_id", &first.id);
        return Ok(Some(first));
    }
    Ok(None)
}

pub fn set_active_workspace_db(db_path: &PathBuf, id: &str) -> Result<(), String> {
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();

    conn.execute(
        "UPDATE workspaces SET updated_at_ms = ? WHERE id = ?",
        params![ts, id],
    )
    .map_err(|e| e.to_string())?;

    crate::db::set_preference(db_path, "active_workspace_id", id)?;
    Ok(())
}

pub fn rename_workspace_db(db_path: &PathBuf, id: &str, new_name: &str) -> Result<(), String> {
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();
    conn.execute(
        "UPDATE workspaces SET name = ?, updated_at_ms = ? WHERE id = ?",
        params![new_name, ts, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn update_workspace_description_db(db_path: &PathBuf, id: &str, description: Option<&str>) -> Result<(), String> {
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();
    conn.execute(
        "UPDATE workspaces SET description = ?, updated_at_ms = ? WHERE id = ?",
        params![description, ts, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_workspace_environments_db(
    db_path: &PathBuf,
    workspace_id: &str,
) -> Result<Vec<Environment>, String> {
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    ensure_schema_columns(&conn);
    let active_id: Option<String> = conn
        .query_row(
            "SELECT active_environment_id FROM workspaces WHERE id = ?",
            [workspace_id],
            |row| row.get(0),
        )
        .ok()
        .flatten();

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, name, is_active, variables_json, created_at_ms, updated_at_ms FROM environments WHERE workspace_id = ? ORDER BY created_at_ms ASC")
        .map_err(|e| e.to_string())?;

    let envs = stmt
        .query_map([workspace_id], |row| {
            let id: String = row.get(0)?;
            let vars_json: String = row.get(4)?;
            let variables: Vec<EnvironmentVariable> = serde_json::from_str(&vars_json).unwrap_or_default();
            let is_active_int: i32 = row.get(3)?;
            let is_active = match &active_id {
                Some(aid) => &id == aid,
                None => is_active_int == 1,
            };

            Ok(Environment {
                id,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                is_active,
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
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();
    let vars_json = serde_json::to_string(&env.variables).unwrap_or_else(|_| "[]".to_string());
    let is_active_int = if env.is_active { 1 } else { 0 };

    if env.is_active {
        let _ = conn.execute(
            "UPDATE environments SET is_active = 0 WHERE workspace_id = ?",
            params![env.workspace_id],
        );
        let _ = conn.execute(
            "UPDATE workspaces SET active_environment_id = ?, updated_at_ms = ? WHERE id = ?",
            params![env.id, ts, env.workspace_id],
        );
    } else {
        let _ = conn.execute(
            "UPDATE workspaces SET active_environment_id = NULL, updated_at_ms = ? WHERE id = ? AND active_environment_id = ?",
            params![ts, env.workspace_id, env.id],
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
    let mut conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
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

pub fn get_workspace_environment_db(
    db_path: &PathBuf,
    id: &str,
) -> Result<Option<Environment>, String> {
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    ensure_schema_columns(&conn);
    let mut stmt = conn
        .prepare("SELECT e.id, e.workspace_id, e.name, e.is_active, e.variables_json, e.created_at_ms, e.updated_at_ms, w.active_environment_id FROM environments e LEFT JOIN workspaces w ON e.workspace_id = w.id WHERE e.id = ?")
        .map_err(|e| e.to_string())?;

    let mut rows = stmt
        .query_map([id], |row| {
            let env_id: String = row.get(0)?;
            let vars_json: String = row.get(4)?;
            let variables: Vec<EnvironmentVariable> = serde_json::from_str(&vars_json).unwrap_or_default();
            let is_active_int: i32 = row.get(3)?;
            let active_id: Option<String> = row.get(7)?;
            let is_active = match &active_id {
                Some(aid) => &env_id == aid,
                None => is_active_int == 1,
            };

            Ok(Environment {
                id: env_id,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                is_active,
                variables,
                created_at_ms: row.get(5)?,
                updated_at_ms: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;

    if let Some(r) = rows.next() {
        r.map(Some).map_err(|e| e.to_string())
    } else {
        Ok(None)
    }
}

pub fn set_active_environment_db(
    db_path: &PathBuf,
    workspace_id: &str,
    env_id: Option<&str>,
) -> Result<(), String> {
    let mut conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let ts = now_ms();

    if let Some(id) = env_id {
        tx.execute(
            "UPDATE environments SET is_active = CASE WHEN id = ? THEN 1 ELSE 0 END WHERE workspace_id = ?",
            params![id, workspace_id],
        )
        .map_err(|e| e.to_string())?;

        tx.execute(
            "UPDATE workspaces SET active_environment_id = ?, updated_at_ms = ? WHERE id = ?",
            params![id, ts, workspace_id],
        )
        .map_err(|e| e.to_string())?;
    } else {
        tx.execute(
            "UPDATE environments SET is_active = 0 WHERE workspace_id = ?",
            params![workspace_id],
        )
        .map_err(|e| e.to_string())?;

        tx.execute(
            "UPDATE workspaces SET active_environment_id = NULL, updated_at_ms = ? WHERE id = ?",
            params![ts, workspace_id],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())
}

pub fn duplicate_workspace_environment_db(
    db_path: &PathBuf,
    id: &str,
    new_name: Option<&str>,
) -> Result<Environment, String> {
    let src = get_workspace_environment_db(db_path, id)?
        .ok_or_else(|| format!("Environment {} not found", id))?;
    let now = now_ms();
    let name = match new_name {
        Some(n) => n.to_string(),
        None => format!("{} (Copy)", src.name),
    };
    let new_env = Environment {
        id: Uuid::new_v4().to_string(),
        workspace_id: src.workspace_id,
        name,
        is_active: false,
        variables: src.variables,
        created_at_ms: now,
        updated_at_ms: now,
    };
    save_workspace_environment_db(db_path, new_env.clone())?;
    Ok(new_env)
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
            if u.is_multiple_of(2) {
                Some("true".to_string())
            } else {
                Some("false".to_string())
            }
        }
        _ => None,
    }
}

/// `$totp:adminSecret` → `adminSecret`.
fn totp_ref_name(key: &str) -> Option<&str> {
    let key = key.trim();
    let prefix = key.get(..6)?;
    prefix.eq_ignore_ascii_case("$totp:").then(|| key[6..].trim())
}

fn totp_code_from_secret(secret: &str) -> Option<String> {
    let params = crate::utils::totp::parse_secret(secret).ok()?;
    Some(crate::utils::totp::code_for_send(&params, crate::utils::totp::now_unix()))
}

fn env_var_value<'a>(env: &'a Environment, name: &str) -> Option<&'a str> {
    let name = name.trim();
    env.variables
        .iter()
        .find(|v| v.enabled && v.key.trim() == name)
        .or_else(|| env.variables.iter().find(|v| v.enabled && v.key.trim().eq_ignore_ascii_case(name)))
        .map(|v| v.current_value())
}

/// The TOTP parameters behind a `$totp:<var>` reference, for showing a live code.
pub fn totp_params_in_env(env: Option<&Environment>, key: &str) -> Option<crate::utils::totp::TotpParams> {
    let name = totp_ref_name(key)?;
    crate::utils::totp::parse_secret(env_var_value(env?, name)?).ok()
}

/// Where `{{$cred:entry.field}}` refs are looked up.
#[derive(Debug, Clone, Copy)]
pub struct CredentialScope<'a> {
    pub db_path: &'a Path,
    pub workspace_id: &'a str,
}

/// What an interpolation substituted from credentials, so the logged copy of a request can be masked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Interpolation {
    pub secrets: Vec<String>,
    /// A `$cred:` ref stayed unresolved because credentials are locked.
    pub locked: bool,
}

pub const MASK: &str = "••••";

impl Interpolation {
    /// Replaces every secret this interpolation substituted, raw or URL-encoded, so it can be logged.
    pub fn mask(&self, text: &str) -> String {
        if self.secrets.is_empty() {
            return text.to_string();
        }
        let mut needles: Vec<String> = self
            .secrets
            .iter()
            .flat_map(|s| {
                let form: String = url::form_urlencoded::byte_serialize(s.as_bytes()).collect();
                [s.clone(), urlencoding::encode(s).into_owned(), form]
            })
            .filter(|n| !n.is_empty())
            .collect();
        needles.sort_by_key(|n| std::cmp::Reverse(n.len()));
        needles.dedup();
        needles.iter().fold(text.to_string(), |out, needle| out.replace(needle.as_str(), MASK))
    }
}

pub fn interpolate_variables_with_env(env: &Environment, text: &str) -> String {
    interpolate(Some(env), None, text, &mut Interpolation::default())
}

/// Fills `{{vars}}` from `env`, credentials from `creds` and dynamic variables, up to 15 levels deep.
pub fn interpolate(
    env: Option<&Environment>,
    creds: Option<CredentialScope>,
    text: &str,
    record: &mut Interpolation,
) -> String {
    if !text.contains("{{") || !text.contains("}}") {
        return text.to_string();
    }

    let mut var_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut lower_var_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for var in env.map(|e| e.variables.as_slice()).unwrap_or_default() {
        if var.enabled && !var.key.trim().is_empty() {
            let k = var.key.trim().to_string();
            let v = var.current_value().to_string();
            lower_var_map.insert(k.to_lowercase(), v.clone());
            var_map.insert(k, v);
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
                let unresolved = || caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string();
                if let Some(val) = var_map.get(key) {
                    changed = true;
                    val.to_string()
                } else if let Some(val) = lower_var_map.get(&key.to_lowercase()) {
                    changed = true;
                    val.to_string()
                } else if let Some(name) = totp_ref_name(key) {
                    let secret = var_map.get(name).or_else(|| lower_var_map.get(&name.to_lowercase()));
                    match secret.and_then(|s| totp_code_from_secret(s)) {
                        Some(code) => {
                            changed = true;
                            code
                        }
                        None => unresolved(),
                    }
                } else if let Some((entry, field)) = crate::credentials::parse_ref(key) {
                    let Some(scope) = creds else { return unresolved() };
                    match crate::credentials::resolve(scope.db_path, scope.workspace_id, entry, field) {
                        crate::credentials::Resolve::Value(value, kind) => {
                            if kind == crate::credentials::FieldKind::Secret && !record.secrets.contains(&value) {
                                record.secrets.push(value.clone());
                            }
                            changed = true;
                            value
                        }
                        crate::credentials::Resolve::Locked => {
                            record.locked = true;
                            unresolved()
                        }
                        crate::credentials::Resolve::Missing => unresolved(),
                    }
                } else if let Some(val) = resolve_dynamic_variable(key) {
                    changed = true;
                    val
                } else {
                    unresolved()
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
    let env = active_environment_db(db_path, workspace_id);
    let scope = CredentialScope { db_path, workspace_id };
    interpolate(env.as_ref(), Some(scope), text, &mut Interpolation::default())
}

pub fn active_environment_db(db_path: &PathBuf, workspace_id: &str) -> Option<Environment> {
    get_workspace_environments_db(db_path, workspace_id)
        .unwrap_or_default()
        .into_iter()
        .find(|e| e.is_active)
}

pub fn find_unresolved_variables(text: &str) -> Vec<Unresolved> {
    if !text.contains("{{") || !text.contains("}}") {
        return Vec::new();
    }
    let re = match regex::Regex::new(r"\{\{\s*([^\r\n{}]+?)\s*\}\}") {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut seen = std::collections::HashSet::new();
    let mut unresolved = Vec::new();
    for caps in re.captures_iter(text) {
        let key = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        if !key.is_empty() && seen.insert(key.to_string()) {
            unresolved.push(Unresolved { name: key.to_string() });
        }
    }
    unresolved
}

pub fn resolve_variables_with_env(
    env: Option<&Environment>,
    text: &str,
) -> (String, Vec<Unresolved>) {
    let resolved = match env {
        Some(e) => interpolate_variables_with_env(e, text),
        None => interpolate_dynamic_variables(text),
    };
    let unresolved = find_unresolved_variables(&resolved);
    (resolved, unresolved)
}

pub fn variable_value_in_env(env: Option<&Environment>, name: &str) -> Option<String> {
    let trimmed = name.trim();
    if let Some(value) = env.and_then(|e| env_var_value(e, trimmed)) {
        return Some(value.to_string());
    }
    if let Some(var) = totp_ref_name(trimmed) {
        return env.and_then(|e| env_var_value(e, var)).and_then(totp_code_from_secret);
    }
    resolve_dynamic_variable(trimmed)
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

    fn env_with(vars: &[(&str, &str)]) -> Environment {
        Environment {
            id: "env-totp".to_string(),
            workspace_id: "ws-1".to_string(),
            name: "Totp".to_string(),
            is_active: true,
            variables: vars
                .iter()
                .map(|(k, v)| EnvironmentVariable {
                    key: k.to_string(),
                    value: v.to_string(),
                    enabled: true,
                    r#type: "secret".to_string(),
                    active_index: 0,
                    variants: vec![],
                })
                .collect(),
            created_at_ms: 0,
            updated_at_ms: 0,
        }
    }

    fn expected_code(secret: &str) -> [String; 2] {
        let p = crate::utils::totp::parse_secret(secret).unwrap();
        let now = crate::utils::totp::now_unix();
        [crate::utils::totp::code_for_send(&p, now), crate::utils::totp::code_for_send(&p, now + 1)]
    }

    #[test]
    fn totp_ref_resolves_base32_and_otpauth_secrets() {
        let base32 = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        let uri = "otpauth://totp/x?secret=MZXW6&digits=8";
        let env = env_with(&[("adminTotpSecret", base32), ("uriSecret", uri)]);

        let code = interpolate_variables_with_env(&env, r#"{"code": "{{$totp:adminTotpSecret}}"}"#);
        let code = code.trim_start_matches(r#"{"code": ""#).trim_end_matches(r#""}"#);
        assert!(expected_code(base32).iter().any(|c| c == code), "{code}");

        let code = interpolate_variables_with_env(&env, "{{ $TOTP:URISECRET }}");
        assert_eq!(code.len(), 8);
        assert!(expected_code(uri).contains(&code));
    }

    #[test]
    fn totp_ref_with_unknown_var_or_bad_secret_stays_unresolved() {
        let env = env_with(&[("bad", "not base32!")]);
        assert_eq!(interpolate_variables_with_env(&env, "{{$totp:missing}}"), "{{$totp:missing}}");
        assert_eq!(interpolate_variables_with_env(&env, "{{$totp:bad}}"), "{{$totp:bad}}");
    }

    #[test]
    fn totp_ref_resolves_through_a_variable() {
        let env = env_with(&[("secret", "MZXW6"), ("code", "{{$totp:secret}}")]);
        let code = interpolate_variables_with_env(&env, "{{code}}");
        assert!(expected_code("MZXW6").contains(&code), "{code}");
        assert!(variable_value_in_env(Some(&env), "$totp:secret").is_some());
        assert!(totp_params_in_env(Some(&env), "$totp:secret").is_some());
    }

    #[test]
    fn cred_refs_resolve_directly_and_through_a_variant_link() {
        use crate::credentials::{create_credential, save_credential, Field, FieldKind};
        let db = crate::credentials::tests_support::temp_db();
        let ws = create_workspace_db(&db, "W".into(), None).unwrap().id;
        let mut cred = create_credential(&db, &ws, "dgAdmin").unwrap();
        cred.fields = vec![Field { key: "password".into(), kind: FieldKind::Secret, extracted: false, value: "hunter2".into() }];
        save_credential(&db, &cred).unwrap();

        let mut env = env_with(&[("adminPassword", "")]);
        env.variables[0].variants = vec![VariableVariant { name: "dev".into(), value: "{{$cred:dgAdmin.password}}".into() }];
        let scope = Some(CredentialScope { db_path: &db, workspace_id: &ws });

        let mut record = Interpolation::default();
        let out = interpolate(Some(&env), scope, "p={{adminPassword}}&q={{ $cred:DGADMIN.Password }}", &mut record);
        assert_eq!(out, "p=hunter2&q=hunter2");
        assert_eq!(record.secrets, ["hunter2"]);

        let missing = "{{$cred:dgAdmin.nope}} {{$cred:nobody.password}}";
        assert_eq!(interpolate(Some(&env), scope, missing, &mut Interpolation::default()), missing);
        assert_eq!(interpolate(None, None, "{{$cred:dgAdmin.password}}", &mut record), "{{$cred:dgAdmin.password}}");
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn mask_hides_raw_and_url_encoded_secrets() {
        let record = Interpolation { secrets: vec!["p@ss word".into()], locked: false };
        assert_eq!(record.mask("a=p@ss word&b=p%40ss%20word&c=p%40ss+word"), "a=••••&b=••••&c=••••");
        assert_eq!(Interpolation::default().mask("p@ss"), "p@ss");
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

    #[test]
    fn test_workspace_and_environment_db_crud() {
        let dir = std::env::temp_dir().join(format!("mitm-ws-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("test.db");
        crate::db::init_database(&db, None).unwrap();

        let ws = create_workspace_db(&db, "Test Workspace".to_string(), Some("Desc".to_string())).unwrap();
        assert_eq!(ws.name, "Test Workspace");
        assert_eq!(ws.description.as_deref(), Some("Desc"));

        let list = get_workspaces_db(&db).unwrap();
        assert!(list.iter().any(|w| w.id == ws.id));

        let fetched = get_workspace_db(&db, &ws.id).unwrap().unwrap();
        assert_eq!(fetched.id, ws.id);

        rename_workspace_db(&db, &ws.id, "Renamed WS").unwrap();
        update_workspace_description_db(&db, &ws.id, Some("New Desc")).unwrap();
        let updated = get_workspace_db(&db, &ws.id).unwrap().unwrap();
        assert_eq!(updated.name, "Renamed WS");
        assert_eq!(updated.description.as_deref(), Some("New Desc"));

        set_active_workspace_db(&db, &ws.id).unwrap();
        let active_ws = get_active_workspace_db(&db).unwrap().unwrap();
        assert_eq!(active_ws.id, ws.id);

        let env = Environment {
            id: Uuid::new_v4().to_string(),
            workspace_id: ws.id.clone(),
            name: "Dev".to_string(),
            is_active: true,
            variables: vec![EnvironmentVariable {
                key: "HOST".to_string(),
                value: "localhost".to_string(),
                enabled: true,
                r#type: "default".to_string(),
                active_index: 0,
                variants: vec![],
            }],
            created_at_ms: now_ms(),
            updated_at_ms: now_ms(),
        };
        save_workspace_environment_db(&db, env.clone()).unwrap();

        let envs = get_workspace_environments_db(&db, &ws.id).unwrap();
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0].name, "Dev");

        let fetched_env = get_workspace_environment_db(&db, &env.id).unwrap().unwrap();
        assert_eq!(fetched_env.name, "Dev");

        let dup = duplicate_workspace_environment_db(&db, &env.id, None).unwrap();
        assert_eq!(dup.name, "Dev (Copy)");
        assert!(!dup.is_active);

        set_active_environment_db(&db, &ws.id, Some(&dup.id)).unwrap();
        let active_env = get_workspace_environment_db(&db, &dup.id).unwrap().unwrap();
        assert!(active_env.is_active);
        let orig_env = get_workspace_environment_db(&db, &env.id).unwrap().unwrap();
        assert!(!orig_env.is_active);

        delete_workspace_environment_db(&db, &dup.id).unwrap();
        assert!(get_workspace_environment_db(&db, &dup.id).unwrap().is_none());

        delete_workspace_db(&db, &ws.id).unwrap();
        assert!(get_workspace_db(&db, &ws.id).unwrap().is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_vars_and_variable_value() {
        let env = Environment {
            id: "env-test".to_string(),
            workspace_id: "ws-test".to_string(),
            name: "Test Env".to_string(),
            is_active: true,
            variables: vec![
                EnvironmentVariable {
                    key: "HOST".to_string(),
                    value: "api.example.com".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
                EnvironmentVariable {
                    key: "REGION".to_string(),
                    value: "us-east-1".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 1,
                    variants: vec![
                        VariableVariant { name: "us".to_string(), value: "us-east-1".to_string() },
                        VariableVariant { name: "eu".to_string(), value: "eu-west-1".to_string() },
                    ],
                },
                EnvironmentVariable {
                    key: "DISABLED_KEY".to_string(),
                    value: "secret".to_string(),
                    enabled: false,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
            ],
            created_at_ms: 0,
            updated_at_ms: 0,
        };

        let template = "https://{{HOST}}/{{REGION}}/users/{{user_id}}?disabled={{DISABLED_KEY}}&uuid={{$guid}}";
        let (resolved, unresolved) = resolve_variables_with_env(Some(&env), template);

        assert!(resolved.starts_with("https://api.example.com/eu-west-1/users/{{user_id}}?disabled={{DISABLED_KEY}}&uuid="));
        assert_eq!(unresolved, vec![
            Unresolved { name: "user_id".to_string() },
            Unresolved { name: "DISABLED_KEY".to_string() },
        ]);

        assert_eq!(variable_value_in_env(Some(&env), "HOST"), Some("api.example.com".to_string()));
        assert_eq!(variable_value_in_env(Some(&env), "host"), Some("api.example.com".to_string()));
        assert_eq!(variable_value_in_env(Some(&env), "REGION"), Some("eu-west-1".to_string()));
        assert_eq!(variable_value_in_env(Some(&env), "DISABLED_KEY"), None);
        assert_eq!(variable_value_in_env(Some(&env), "user_id"), None);
        assert!(variable_value_in_env(Some(&env), "$guid").is_some());
    }

    #[test]
    fn test_active_environment_survives_restart() {
        let dir = std::env::temp_dir().join(format!("mitm-env-survive-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("test.db");
        crate::db::init_database(&db, None).unwrap();

        let ws = create_workspace_db(&db, "Main Workspace".to_string(), None).unwrap();
        set_active_workspace_db(&db, &ws.id).unwrap();

        let dev_env = Environment {
            id: "env-dev".to_string(),
            workspace_id: ws.id.clone(),
            name: "Dev".to_string(),
            is_active: true,
            variables: vec![EnvironmentVariable {
                key: "BASE_URL".to_string(),
                value: "https://dev.example.com".to_string(),
                enabled: true,
                r#type: "default".to_string(),
                active_index: 0,
                variants: vec![],
            }],
            created_at_ms: now_ms(),
            updated_at_ms: now_ms(),
        };
        save_workspace_environment_db(&db, dev_env).unwrap();

        let prod_env = Environment {
            id: "env-prod".to_string(),
            workspace_id: ws.id.clone(),
            name: "Prod".to_string(),
            is_active: false,
            variables: vec![EnvironmentVariable {
                key: "BASE_URL".to_string(),
                value: "https://prod.example.com".to_string(),
                enabled: true,
                r#type: "default".to_string(),
                active_index: 0,
                variants: vec![],
            }],
            created_at_ms: now_ms() + 1,
            updated_at_ms: now_ms() + 1,
        };
        save_workspace_environment_db(&db, prod_env).unwrap();

        set_active_environment_db(&db, &ws.id, Some("env-prod")).unwrap();

        let reloaded_ws = get_active_workspace_db(&db).unwrap().unwrap();
        assert_eq!(reloaded_ws.active_environment_id.as_deref(), Some("env-prod"));

        let envs = get_workspace_environments_db(&db, &reloaded_ws.id).unwrap();
        let active_env = envs.iter().find(|e| e.is_active).expect("active env found");
        assert_eq!(active_env.id, "env-prod");
        assert_eq!(active_env.name, "Prod");

        let resolved = interpolate_variables(&db, &reloaded_ws.id, "{{BASE_URL}}/api");
        assert_eq!(resolved, "https://prod.example.com/api");

        set_active_environment_db(&db, &ws.id, None).unwrap();
        let envs_none = get_workspace_environments_db(&db, &reloaded_ws.id).unwrap();
        assert!(envs_none.iter().all(|e| !e.is_active));

        let _ = std::fs::remove_dir_all(&dir);
    }
}

