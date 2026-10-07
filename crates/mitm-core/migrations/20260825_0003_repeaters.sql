CREATE TABLE IF NOT EXISTS repeaters (
    id TEXT PRIMARY KEY,
    method TEXT NOT NULL DEFAULT 'GET',
    url TEXT NOT NULL,
    headers_json TEXT NOT NULL DEFAULT '[]',
    params_json TEXT NOT NULL DEFAULT '[]',
    body_type TEXT NOT NULL DEFAULT 'none',
    body_content TEXT,
    extract_rules_json TEXT NOT NULL DEFAULT '[]',
    order_index INTEGER NOT NULL DEFAULT 0,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS repeater_histories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repeater_id TEXT NOT NULL,
    method TEXT NOT NULL,
    url TEXT NOT NULL,
    request_headers_json TEXT NOT NULL,
    request_body TEXT,
    status_code INTEGER NOT NULL,
    response_headers_json TEXT NOT NULL,
    response_body TEXT,
    duration_ms INTEGER NOT NULL,
    executed_at_ms INTEGER NOT NULL,
    FOREIGN KEY(repeater_id) REFERENCES repeaters(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_repeater_histories_rep_id ON repeater_histories(repeater_id);
CREATE INDEX IF NOT EXISTS idx_repeater_histories_executed ON repeater_histories(executed_at_ms DESC);
