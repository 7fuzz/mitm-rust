CREATE TABLE IF NOT EXISTS history (
    id TEXT PRIMARY KEY,
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
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_history_created_at ON history(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_history_method ON history(method);
CREATE INDEX IF NOT EXISTS idx_history_status ON history(status_code);
CREATE INDEX IF NOT EXISTS idx_history_host ON history(host);

CREATE TABLE IF NOT EXISTS intercept_rules (
    id TEXT PRIMARY KEY,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    target_phase TEXT NOT NULL,
    match_field TEXT NOT NULL,
    operator TEXT NOT NULL,
    match_value TEXT NOT NULL,
    order_index INTEGER NOT NULL DEFAULT 0,
    created_at_ms INTEGER NOT NULL,
    action TEXT NOT NULL DEFAULT 'intercept'
);

CREATE TABLE IF NOT EXISTS app_preferences (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO app_preferences (key, value) VALUES ('history_limiter_enabled', 'true');
INSERT OR IGNORE INTO app_preferences (key, value) VALUES ('history_limiter_max_rows', '500');
