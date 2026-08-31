-- Migration 20260831_0008_rewrite_rules.sql
CREATE TABLE IF NOT EXISTS rewrite_rules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    action_type TEXT NOT NULL, -- 'partial_request' | 'full_request' | 'partial_response' | 'full_response'
    
    -- Match Filter
    match_field TEXT NOT NULL DEFAULT 'all', -- 'all' | 'url' | 'host' | 'path' | 'method' | 'header'
    match_operator TEXT NOT NULL DEFAULT 'contains', -- 'contains' | 'equals' | 'regex' | 'starts_with'
    match_value TEXT NOT NULL DEFAULT '',
    
    -- Transformation
    target_part TEXT NOT NULL DEFAULT 'url', -- 'url' | 'query' | 'header' | 'body' | 'method' | 'status'
    target_header TEXT,
    match_pattern TEXT NOT NULL DEFAULT '',
    replacement_value TEXT NOT NULL DEFAULT '',
    is_regex INTEGER NOT NULL DEFAULT 0,
    
    -- Full Mock Response
    mock_status_code INTEGER,
    mock_headers_json TEXT,
    mock_body TEXT,
    
    order_index INTEGER NOT NULL DEFAULT 0,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_rewrite_rules_order ON rewrite_rules(order_index ASC);

CREATE TABLE IF NOT EXISTS rewrite_history (
    id TEXT PRIMARY KEY,
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

CREATE INDEX IF NOT EXISTS idx_rewrite_history_created ON rewrite_history(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_rewrite_history_rule ON rewrite_history(rule_id);
