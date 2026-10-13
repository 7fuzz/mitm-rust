use rusqlite::{params, Connection};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::repeater::REPEATER_WORKSPACE_ID;
use crate::db::Db;
use super::model::*;

pub(super) fn now_ms() -> i64 {
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

pub fn get_workspaces_db(db: &Db) -> Result<Vec<Workspace>, String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
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
    db: &Db,
    name: String,
    description: Option<String>,
) -> Result<Workspace, String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
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

pub fn update_workspace_db(db: &Db, workspace: Workspace) -> Result<(), String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
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

pub fn delete_workspace_db(db: &Db, id: &str) -> Result<(), String> {
    if id == REPEATER_WORKSPACE_ID {
        return Err("The Repeater workspace can't be deleted.".to_string());
    }
    let conn = db.conn().map_err(|e| e.to_string())?;
    let _ = conn.execute("DELETE FROM environments WHERE workspace_id = ?", params![id]);

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

pub fn get_workspace_db(db: &Db, id: &str) -> Result<Option<Workspace>, String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
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

pub fn get_active_workspace_db(db: &Db) -> Result<Option<Workspace>, String> {
    if let Some(active_id) = crate::db::get_preference(db, "active_workspace_id") {
        if let Ok(Some(ws)) = get_workspace_db(db, &active_id) {
            return Ok(Some(ws));
        }
    }
    let all = get_workspaces_db(db)?;
    if let Some(first) = all.into_iter().next() {
        let _ = crate::db::set_preference(db, "active_workspace_id", &first.id);
        return Ok(Some(first));
    }
    Ok(None)
}

pub fn set_active_workspace_db(db: &Db, id: &str) -> Result<(), String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let ts = now_ms();

    conn.execute(
        "UPDATE workspaces SET updated_at_ms = ? WHERE id = ?",
        params![ts, id],
    )
    .map_err(|e| e.to_string())?;

    crate::db::set_preference(db, "active_workspace_id", id)?;
    Ok(())
}

pub fn rename_workspace_db(db: &Db, id: &str, new_name: &str) -> Result<(), String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let ts = now_ms();
    conn.execute(
        "UPDATE workspaces SET name = ?, updated_at_ms = ? WHERE id = ?",
        params![new_name, ts, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn update_workspace_description_db(db: &Db, id: &str, description: Option<&str>) -> Result<(), String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let ts = now_ms();
    conn.execute(
        "UPDATE workspaces SET description = ?, updated_at_ms = ? WHERE id = ?",
        params![description, ts, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_workspace_environments_db(
    db: &Db,
    workspace_id: &str,
) -> Result<Vec<Environment>, String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
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
    db: &Db,
    env: Environment,
) -> Result<(), String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
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
pub fn delete_workspace_environment_db(db: &Db, id: &str) -> Result<(), String> {
    let mut conn = db.conn().map_err(|e| e.to_string())?;
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
    db: &Db,
    id: &str,
) -> Result<Option<Environment>, String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
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
    db: &Db,
    workspace_id: &str,
    env_id: Option<&str>,
) -> Result<(), String> {
    let mut conn = db.conn().map_err(|e| e.to_string())?;
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
    db: &Db,
    id: &str,
    new_name: Option<&str>,
) -> Result<Environment, String> {
    let src = get_workspace_environment_db(db, id)?
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
    save_workspace_environment_db(db, new_env.clone())?;
    Ok(new_env)
}
