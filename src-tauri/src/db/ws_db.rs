use std::path::PathBuf;
use rusqlite::{params, Connection};
use crate::state::{WebSocketConn, WebSocketMessage};

pub fn load_ws_connections(db_path: &PathBuf) -> Result<Vec<WebSocketConn>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, url, status, handshake_time, closed_at, protocol, client_addr, is_client_session, message_count, COALESCE(listener_label, '')
             FROM ws_connections
             ORDER BY handshake_time DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let is_client_session_int: i64 = row.get(7).unwrap_or(0);
            Ok(WebSocketConn {
                connection_id: row.get(0)?,
                url: row.get(1)?,
                status: row.get(2)?,
                handshake_time: row.get(3)?,
                closed_at: row.get(4)?,
                protocol: row.get(5)?,
                client_addr: row.get(6)?,
                is_client_session: is_client_session_int != 0,
                message_count: row.get(8).unwrap_or(0),
                listener_label: row.get(9).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?;

    let mut connections = Vec::new();
    for r in rows {
        if let Ok(c) = r {
            connections.push(c);
        }
    }
    Ok(connections)
}

pub fn save_ws_connection(db_path: &PathBuf, conn_info: &WebSocketConn) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO ws_connections (id, url, status, handshake_time, closed_at, protocol, client_addr, is_client_session, message_count, listener_label)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            conn_info.connection_id,
            conn_info.url,
            conn_info.status,
            conn_info.handshake_time,
            conn_info.closed_at,
            conn_info.protocol,
            conn_info.client_addr,
            if conn_info.is_client_session { 1 } else { 0 },
            conn_info.message_count,
            conn_info.listener_label,
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn update_ws_connection_status(
    db_path: &PathBuf,
    connection_id: &str,
    status: &str,
    closed_at: Option<i64>,
) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE ws_connections SET status = ?, closed_at = ? WHERE id = ?",
        params![status, closed_at, connection_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn increment_ws_message_count(db_path: &PathBuf, connection_id: &str) -> Result<u64, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE ws_connections SET message_count = message_count + 1 WHERE id = ?",
        params![connection_id],
    ).map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT message_count FROM ws_connections WHERE id = ?")
        .map_err(|e| e.to_string())?;
    let count: u64 = stmt
        .query_row(params![connection_id], |row| row.get(0))
        .unwrap_or(0);
    Ok(count)
}

pub fn delete_ws_connection(db_path: &PathBuf, id: &str) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM ws_connections WHERE id = ?", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn save_ws_message(db_path: &PathBuf, msg: &WebSocketMessage) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO ws_messages (id, connection_id, direction, msg_type, payload, timestamp, length, is_injected)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            msg.id,
            msg.connection_id,
            msg.direction,
            msg.msg_type,
            msg.payload,
            msg.timestamp,
            msg.length,
            if msg.is_injected { 1 } else { 0 },
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_ws_messages(
    db_path: &PathBuf,
    connection_id: &str,
    limit: Option<u32>,
) -> Result<Vec<WebSocketMessage>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let limit_val = limit.unwrap_or(1000);
    let mut stmt = conn
        .prepare(
            "SELECT id, connection_id, direction, msg_type, payload, timestamp, length, is_injected
             FROM ws_messages
             WHERE connection_id = ?
             ORDER BY timestamp ASC
             LIMIT ?",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![connection_id, limit_val], |row| {
            let is_injected_int: i64 = row.get(7).unwrap_or(0);
            Ok(WebSocketMessage {
                id: row.get(0)?,
                connection_id: row.get(1)?,
                direction: row.get(2)?,
                msg_type: row.get(3)?,
                payload: row.get(4)?,
                timestamp: row.get(5)?,
                length: row.get(6)?,
                is_injected: is_injected_int != 0,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut messages = Vec::new();
    for r in rows {
        if let Ok(m) = r {
            messages.push(m);
        }
    }
    Ok(messages)
}

pub fn clear_ws_messages(db_path: &PathBuf, connection_id: Option<&str>) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    if let Some(cid) = connection_id {
        conn.execute("DELETE FROM ws_messages WHERE connection_id = ?", params![cid])
            .map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE ws_connections SET message_count = 0 WHERE id = ?",
            params![cid],
        ).map_err(|e| e.to_string())?;
    } else {
        conn.execute("DELETE FROM ws_messages", [])
            .map_err(|e| e.to_string())?;
        conn.execute("UPDATE ws_connections SET message_count = 0", [])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
