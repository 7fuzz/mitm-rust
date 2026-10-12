-- One app-wide data key. kdf NULL means no master password and wrapped_key holds the raw key.
CREATE TABLE IF NOT EXISTS credential_keys (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    kdf TEXT,
    kdf_params TEXT,
    salt BLOB,
    nonce BLOB,
    wrapped_key BLOB NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

-- fields_json lists field keys and kinds in plaintext so pickers and value-less exports work while locked.
-- ciphertext holds the field values and notes.
CREATE TABLE IF NOT EXISTS credentials (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL COLLATE NOCASE,
    fields_json TEXT NOT NULL DEFAULT '[]',
    nonce BLOB NOT NULL,
    ciphertext BLOB NOT NULL,
    order_index INTEGER NOT NULL DEFAULT 0,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    UNIQUE (workspace_id, name)
);

CREATE INDEX IF NOT EXISTS idx_credentials_workspace ON credentials(workspace_id, order_index)
