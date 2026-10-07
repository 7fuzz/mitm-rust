-- Tag WebSocket connections with the listener (source) they arrived on
ALTER TABLE ws_connections ADD COLUMN listener_label TEXT NOT NULL DEFAULT '';
