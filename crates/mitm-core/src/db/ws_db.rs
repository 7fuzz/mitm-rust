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

    Ok(rows.flatten().collect())
}

/// Inserts the connection and returns its new id
pub fn insert_ws_connection(db_path: &PathBuf, conn_info: &WebSocketConn) -> Result<i64, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO ws_connections (url, status, handshake_time, closed_at, protocol, client_addr, is_client_session, message_count, listener_label)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
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
    Ok(conn.last_insert_rowid())
}

pub fn update_ws_connection_status(
    db_path: &PathBuf,
    connection_id: i64,
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

/// No socket outlives the process, so any session still marked connected was cut off by an exit or crash.
pub fn close_stale_ws_connections(db_path: &PathBuf, now: i64) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE ws_connections SET status = 'disconnected', closed_at = COALESCE(closed_at, ?) WHERE status != 'disconnected'",
        params![now],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn increment_ws_message_count(db_path: &PathBuf, connection_id: i64) -> Result<u64, String> {
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

pub fn delete_ws_connection(db_path: &PathBuf, id: i64) -> Result<(), String> {
    let mut conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM ws_messages WHERE connection_id = ?", params![id])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM ws_connections WHERE id = ?", params![id])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// Inserts the message and sets its id to the new row's id
pub fn save_ws_message(db_path: &PathBuf, msg: &mut WebSocketMessage) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO ws_messages (connection_id, direction, msg_type, payload, timestamp, length, is_injected)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![
            msg.connection_id,
            msg.direction,
            msg.msg_type,
            msg.payload,
            msg.timestamp,
            msg.length,
            if msg.is_injected { 1 } else { 0 },
        ],
    ).map_err(|e| e.to_string())?;
    msg.id = conn.last_insert_rowid();
    Ok(())
}

pub fn load_ws_messages(
    db_path: &PathBuf,
    connection_id: i64,
    limit: Option<u32>,
) -> Result<Vec<WebSocketMessage>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let limit_val = limit.unwrap_or(1000);
    let mut stmt = conn
        .prepare(
            "SELECT id, connection_id, direction, msg_type, payload, timestamp, length, is_injected
             FROM ws_messages
             WHERE connection_id = ?
             ORDER BY id ASC
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

    Ok(rows.flatten().collect())
}

pub fn clear_ws_messages(db_path: &PathBuf, connection_id: Option<i64>) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn conn_row(url: &str) -> WebSocketConn {
        WebSocketConn {
            connection_id: 0,
            url: url.to_string(),
            status: "connected".to_string(),
            handshake_time: 0,
            closed_at: None,
            protocol: None,
            client_addr: None,
            is_client_session: true,
            message_count: 0,
            listener_label: String::new(),
        }
    }

    fn msg_row(connection_id: i64) -> WebSocketMessage {
        WebSocketMessage {
            id: 0,
            connection_id,
            direction: "to_server".to_string(),
            msg_type: "text".to_string(),
            payload: "hi".to_string(),
            timestamp: 0,
            length: 2,
            is_injected: true,
        }
    }

    #[test]
    fn clear_resets_count_and_delete_removes_messages() {
        let dir = std::env::temp_dir().join(format!("mitm-ws-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("test.db");
        crate::db::init_database(&db_path, None).unwrap();

        let a = insert_ws_connection(&db_path, &conn_row("wss://a")).unwrap();
        let b = insert_ws_connection(&db_path, &conn_row("wss://b")).unwrap();
        for _ in 0..3 {
            save_ws_message(&db_path, &mut msg_row(a)).unwrap();
            increment_ws_message_count(&db_path, a).unwrap();
        }
        save_ws_message(&db_path, &mut msg_row(b)).unwrap();
        increment_ws_message_count(&db_path, b).unwrap();

        clear_ws_messages(&db_path, Some(a)).unwrap();
        assert!(load_ws_messages(&db_path, a, None).unwrap().is_empty());
        let a_count = load_ws_connections(&db_path).unwrap().iter().find(|c| c.connection_id == a).unwrap().message_count;
        assert_eq!(a_count, 0);
        assert_eq!(load_ws_messages(&db_path, b, None).unwrap().len(), 1, "other session untouched");

        delete_ws_connection(&db_path, b).unwrap();
        assert!(load_ws_messages(&db_path, b, None).unwrap().is_empty());
        assert!(load_ws_connections(&db_path).unwrap().iter().all(|c| c.connection_id != b));

        close_stale_ws_connections(&db_path, 99).unwrap();
        let a_row = load_ws_connections(&db_path).unwrap().into_iter().find(|c| c.connection_id == a).unwrap();
        assert_eq!((a_row.status.as_str(), a_row.closed_at), ("disconnected", Some(99)));

        std::fs::remove_dir_all(&dir).ok();
    }
}
