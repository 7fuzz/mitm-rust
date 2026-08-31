use std::path::PathBuf;
use rusqlite::{params, Connection};
use crate::state::{WebhookDelivery, WebhookEndpoint};

pub fn load_webhook_endpoints(db_path: &PathBuf) -> Result<Vec<WebhookEndpoint>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, path, secret_key, created_at, hit_count FROM webhook_endpoints ORDER BY created_at ASC")
        .map_err(|e| e.to_string())?;

    let endpoints = stmt
        .query_map([], |row| {
            Ok(WebhookEndpoint {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                secret_key: row.get(3)?,
                created_at: row.get(4)?,
                hit_count: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(endpoints)
}

pub fn get_webhook_endpoint_by_path(db_path: &PathBuf, path: &str) -> Result<Option<WebhookEndpoint>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, path, secret_key, created_at, hit_count FROM webhook_endpoints WHERE path = ? LIMIT 1")
        .map_err(|e| e.to_string())?;

    let mut rows = stmt
        .query_map([path], |row| {
            Ok(WebhookEndpoint {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                secret_key: row.get(3)?,
                created_at: row.get(4)?,
                hit_count: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;

    if let Some(row) = rows.next() {
        row.map(Some).map_err(|e| e.to_string())
    } else {
        Ok(None)
    }
}

pub fn save_webhook_endpoint(db_path: &PathBuf, endpoint: &WebhookEndpoint) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO webhook_endpoints (id, name, path, secret_key, created_at, hit_count)
         VALUES (?, ?, ?, ?, ?, ?)",
        params![
            endpoint.id,
            endpoint.name,
            endpoint.path,
            endpoint.secret_key,
            endpoint.created_at,
            endpoint.hit_count,
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_webhook_endpoint(db_path: &PathBuf, id: &str) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM webhook_endpoints WHERE id = ?", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM webhook_deliveries WHERE endpoint_id = ?", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn increment_webhook_endpoint_hits(db_path: &PathBuf, id: &str) -> Result<u64, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE webhook_endpoints SET hit_count = hit_count + 1 WHERE id = ?",
        params![id],
    ).map_err(|e| e.to_string())?;

    let hit_count: u64 = conn.query_row(
        "SELECT hit_count FROM webhook_endpoints WHERE id = ?",
        params![id],
        |row| row.get(0),
    ).unwrap_or(0);

    Ok(hit_count)
}

pub fn load_webhook_deliveries(db_path: &PathBuf, limit: Option<u32>) -> Result<Vec<WebhookDelivery>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let limit_val = limit.unwrap_or(500);

    let mut stmt = conn
        .prepare("SELECT id, endpoint_id, endpoint_path, headers, payload, signature_status, computed_hmac, provided_hmac, timestamp 
                  FROM webhook_deliveries ORDER BY timestamp DESC LIMIT ?")
        .map_err(|e| e.to_string())?;

    let deliveries = stmt
        .query_map(params![limit_val], |row| {
            let headers_json: String = row.get(3)?;
            let headers: Vec<(String, String)> = serde_json::from_str(&headers_json).unwrap_or_default();

            Ok(WebhookDelivery {
                id: row.get(0)?,
                endpoint_id: row.get(1)?,
                endpoint_path: row.get(2)?,
                headers,
                payload: row.get(4)?,
                signature_status: row.get(5)?,
                computed_hmac: row.get(6)?,
                provided_hmac: row.get(7)?,
                timestamp: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(deliveries)
}

pub fn save_webhook_delivery(db_path: &PathBuf, delivery: &WebhookDelivery) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let headers_json = serde_json::to_string(&delivery.headers).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT INTO webhook_deliveries (id, endpoint_id, endpoint_path, headers, payload, signature_status, computed_hmac, provided_hmac, timestamp)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            delivery.id,
            delivery.endpoint_id,
            delivery.endpoint_path,
            headers_json,
            delivery.payload,
            delivery.signature_status,
            delivery.computed_hmac,
            delivery.provided_hmac,
            delivery.timestamp,
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn clear_webhook_deliveries(db_path: &PathBuf) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM webhook_deliveries", []).map_err(|e| e.to_string())?;
    Ok(())
}
