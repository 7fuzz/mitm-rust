CREATE TABLE IF NOT EXISTS workspaces (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  active_environment_id TEXT,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS environments (
  id TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL,
  name TEXT NOT NULL,
  is_active INTEGER NOT NULL DEFAULT 0,
  variables_json TEXT NOT NULL DEFAULT '[]',
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL,
  FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS collections (
  id TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL,
  parent_id TEXT,
  name TEXT NOT NULL,
  description TEXT,
  order_index INTEGER NOT NULL DEFAULT 0,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL,
  FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
  FOREIGN KEY(parent_id) REFERENCES collections(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS requests (
  id TEXT PRIMARY KEY,
  collection_id TEXT NOT NULL,
  name TEXT NOT NULL,
  method TEXT NOT NULL DEFAULT 'GET',
  url TEXT NOT NULL,
  headers_json TEXT NOT NULL DEFAULT '[]',
  params_json TEXT NOT NULL DEFAULT '[]',
  body_type TEXT NOT NULL DEFAULT 'none',
  body_content TEXT,
  extract_rules_json TEXT NOT NULL DEFAULT '[]',
  description TEXT,
  order_index INTEGER NOT NULL DEFAULT 0,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL,
  FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS request_histories (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  request_id TEXT NOT NULL,
  method TEXT NOT NULL,
  url TEXT NOT NULL,
  request_headers_json TEXT NOT NULL,
  request_body TEXT,
  status_code INTEGER NOT NULL,
  response_headers_json TEXT NOT NULL,
  response_body TEXT,
  duration_ms INTEGER NOT NULL,
  executed_at_ms INTEGER NOT NULL,
  FOREIGN KEY(request_id) REFERENCES requests(id) ON DELETE CASCADE
);
