-- Migration: 20260831_0011_websockets.sql
-- Create WebSocket connections and message stream persistence tables

CREATE TABLE IF NOT EXISTS ws_connections (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    status TEXT NOT NULL,
    handshake_time INTEGER NOT NULL,
    closed_at INTEGER,
    protocol TEXT,
    client_addr TEXT,
    is_client_session INTEGER NOT NULL DEFAULT 0,
    message_count INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_ws_connections_handshake_time ON ws_connections(handshake_time DESC);

CREATE TABLE IF NOT EXISTS ws_messages (
    id TEXT PRIMARY KEY,
    connection_id TEXT NOT NULL,
    direction TEXT NOT NULL,
    msg_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    length INTEGER NOT NULL,
    is_injected INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(connection_id) REFERENCES ws_connections(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ws_messages_connection_id ON ws_messages(connection_id);
CREATE INDEX IF NOT EXISTS idx_ws_messages_timestamp ON ws_messages(timestamp ASC);
