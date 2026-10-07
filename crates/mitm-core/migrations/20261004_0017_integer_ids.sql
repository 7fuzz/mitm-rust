-- Switch high-volume local tables from TEXT (uuid) to INTEGER primary keys.
-- Rows are renumbered in their original order. Children are copied before the old parents are
-- dropped, because foreign_keys = ON would otherwise cascade-delete them.

CREATE TABLE history_new (
    id INTEGER PRIMARY KEY,
    method TEXT NOT NULL,
    url TEXT NOT NULL,
    host TEXT NOT NULL,
    path TEXT NOT NULL DEFAULT '/',
    content_type TEXT NOT NULL DEFAULT '-',
    response_size INTEGER NOT NULL DEFAULT 0,
    status_code INTEGER NOT NULL,
    request_headers TEXT NOT NULL,
    response_headers TEXT NOT NULL,
    request_body TEXT NOT NULL,
    response_body TEXT NOT NULL,
    phase TEXT NOT NULL,
    duration_ms INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    is_intercepted INTEGER NOT NULL DEFAULT 0,
    is_rewritten INTEGER NOT NULL DEFAULT 0,
    is_failed INTEGER NOT NULL DEFAULT 0,
    listener_label TEXT NOT NULL DEFAULT '',
    request_at TEXT,
    response_at TEXT
);

INSERT INTO history_new (method, url, host, path, content_type, response_size, status_code, request_headers, response_headers, request_body, response_body, phase, duration_ms, created_at, is_intercepted, is_rewritten, is_failed, listener_label, request_at, response_at)
SELECT method, url, host, path, content_type, response_size, status_code, request_headers, response_headers, request_body, response_body, phase, duration_ms, created_at, is_intercepted, is_rewritten, is_failed, listener_label, request_at, response_at
FROM history ORDER BY CAST(id AS INTEGER), created_at, rowid;

DROP TABLE history;
ALTER TABLE history_new RENAME TO history;
CREATE INDEX idx_history_created_at ON history(created_at DESC);
CREATE INDEX idx_history_method ON history(method);
CREATE INDEX idx_history_status ON history(status_code);
CREATE INDEX idx_history_host ON history(host);

CREATE TABLE rewrite_history_new (
    id INTEGER PRIMARY KEY,
    rule_id TEXT,
    rule_name TEXT NOT NULL,
    action_type TEXT NOT NULL,
    method TEXT NOT NULL,
    original_url TEXT NOT NULL,
    rewritten_url TEXT NOT NULL,
    original_headers TEXT,
    rewritten_headers TEXT,
    original_body TEXT,
    rewritten_body TEXT,
    status_code INTEGER,
    duration_ms INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO rewrite_history_new (rule_id, rule_name, action_type, method, original_url, rewritten_url, original_headers, rewritten_headers, original_body, rewritten_body, status_code, duration_ms, created_at)
SELECT rule_id, rule_name, action_type, method, original_url, rewritten_url, original_headers, rewritten_headers, original_body, rewritten_body, status_code, duration_ms, created_at
FROM rewrite_history ORDER BY created_at, rowid;

DROP TABLE rewrite_history;
ALTER TABLE rewrite_history_new RENAME TO rewrite_history;
CREATE INDEX idx_rewrite_history_created ON rewrite_history(created_at DESC);
CREATE INDEX idx_rewrite_history_rule ON rewrite_history(rule_id);

CREATE TABLE webhook_deliveries_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    endpoint_id TEXT NOT NULL,
    endpoint_path TEXT NOT NULL,
    headers TEXT NOT NULL,
    payload TEXT NOT NULL,
    signature_status TEXT NOT NULL,
    computed_hmac TEXT,
    provided_hmac TEXT,
    timestamp INTEGER NOT NULL
);

INSERT INTO webhook_deliveries_new (endpoint_id, endpoint_path, headers, payload, signature_status, computed_hmac, provided_hmac, timestamp)
SELECT endpoint_id, endpoint_path, headers, payload, signature_status, computed_hmac, provided_hmac, timestamp
FROM webhook_deliveries ORDER BY timestamp, rowid;

DROP TABLE webhook_deliveries;
ALTER TABLE webhook_deliveries_new RENAME TO webhook_deliveries;
CREATE INDEX idx_webhook_deliveries_endpoint ON webhook_deliveries(endpoint_id);
CREATE INDEX idx_webhook_deliveries_timestamp ON webhook_deliveries(timestamp DESC);

CREATE TABLE _ws_conn_map AS
SELECT id AS old_id, ROW_NUMBER() OVER (ORDER BY handshake_time, rowid) AS new_id FROM ws_connections;

CREATE TABLE ws_connections_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL,
    status TEXT NOT NULL,
    handshake_time INTEGER NOT NULL,
    closed_at INTEGER,
    protocol TEXT,
    client_addr TEXT,
    is_client_session INTEGER NOT NULL DEFAULT 0,
    message_count INTEGER NOT NULL DEFAULT 0,
    listener_label TEXT NOT NULL DEFAULT ''
);

CREATE TABLE ws_messages_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    connection_id INTEGER NOT NULL,
    direction TEXT NOT NULL,
    msg_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    length INTEGER NOT NULL,
    is_injected INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(connection_id) REFERENCES ws_connections_new(id) ON DELETE CASCADE
);

INSERT INTO ws_connections_new (id, url, status, handshake_time, closed_at, protocol, client_addr, is_client_session, message_count, listener_label)
SELECT m.new_id, c.url, c.status, c.handshake_time, c.closed_at, c.protocol, c.client_addr, c.is_client_session, c.message_count, c.listener_label
FROM ws_connections c JOIN _ws_conn_map m ON m.old_id = c.id;

INSERT INTO ws_messages_new (connection_id, direction, msg_type, payload, timestamp, length, is_injected)
SELECT m.new_id, w.direction, w.msg_type, w.payload, w.timestamp, w.length, w.is_injected
FROM ws_messages w JOIN _ws_conn_map m ON m.old_id = w.connection_id
ORDER BY w.timestamp, w.rowid;

DROP TABLE _ws_conn_map;
DROP TABLE ws_messages;
DROP TABLE ws_connections;
ALTER TABLE ws_connections_new RENAME TO ws_connections;
ALTER TABLE ws_messages_new RENAME TO ws_messages;
CREATE INDEX idx_ws_connections_handshake_time ON ws_connections(handshake_time DESC);
CREATE INDEX idx_ws_messages_connection_id ON ws_messages(connection_id);
CREATE INDEX idx_ws_messages_timestamp ON ws_messages(timestamp ASC);

CREATE TABLE _fuzz_run_map AS
SELECT id AS old_id, ROW_NUMBER() OVER (ORDER BY created_at_ms, rowid) AS new_id FROM fuzz_runs;

CREATE TABLE fuzz_runs_new (
    id            INTEGER PRIMARY KEY,
    name          TEXT NOT NULL,
    attack_type   TEXT NOT NULL,
    config_json   TEXT NOT NULL,
    total         INTEGER NOT NULL DEFAULT 0,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE fuzz_results_new (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id                INTEGER NOT NULL,
    idx                   INTEGER NOT NULL,
    payloads_json         TEXT NOT NULL,
    method                TEXT NOT NULL,
    url                   TEXT NOT NULL,
    request_headers_json  TEXT NOT NULL,
    request_body          TEXT,
    status_code           INTEGER NOT NULL,
    response_headers_json TEXT NOT NULL,
    response_body         TEXT,
    response_size         INTEGER NOT NULL DEFAULT 0,
    duration_ms           INTEGER NOT NULL DEFAULT 0,
    error                 TEXT,
    FOREIGN KEY (run_id) REFERENCES fuzz_runs_new(id) ON DELETE CASCADE
);

INSERT INTO fuzz_runs_new (id, name, attack_type, config_json, total, created_at_ms)
SELECT m.new_id, r.name, r.attack_type, r.config_json, r.total, r.created_at_ms
FROM fuzz_runs r JOIN _fuzz_run_map m ON m.old_id = r.id;

INSERT INTO fuzz_results_new (id, run_id, idx, payloads_json, method, url, request_headers_json, request_body, status_code, response_headers_json, response_body, response_size, duration_ms, error)
SELECT f.id, m.new_id, f.idx, f.payloads_json, f.method, f.url, f.request_headers_json, f.request_body, f.status_code, f.response_headers_json, f.response_body, f.response_size, f.duration_ms, f.error
FROM fuzz_results f JOIN _fuzz_run_map m ON m.old_id = f.run_id;

DROP TABLE _fuzz_run_map;
DROP TABLE fuzz_results;
DROP TABLE fuzz_runs;
ALTER TABLE fuzz_runs_new RENAME TO fuzz_runs;
ALTER TABLE fuzz_results_new RENAME TO fuzz_results;
CREATE INDEX idx_fuzz_results_run ON fuzz_results(run_id, idx)
