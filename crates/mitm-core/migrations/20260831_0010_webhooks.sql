CREATE TABLE IF NOT EXISTS webhook_endpoints (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    secret_key TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    hit_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS webhook_deliveries (
    id TEXT PRIMARY KEY,
    endpoint_id TEXT NOT NULL,
    endpoint_path TEXT NOT NULL,
    headers TEXT NOT NULL,
    payload TEXT NOT NULL,
    signature_status TEXT NOT NULL,
    computed_hmac TEXT,
    provided_hmac TEXT,
    timestamp INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_endpoint ON webhook_deliveries(endpoint_id);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_timestamp ON webhook_deliveries(timestamp DESC);
