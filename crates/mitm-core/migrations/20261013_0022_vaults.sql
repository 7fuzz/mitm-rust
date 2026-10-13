-- Credentials live in vaults. A vault is linked to any number of workspaces, or none, and refs name it:
-- {{$cred:vault/entry.field}}. Moving existing credentials into vaults happens in Rust (see credentials::vault).
CREATE TABLE IF NOT EXISTS vaults (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    order_index INTEGER NOT NULL DEFAULT 0,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS vault_workspaces (
    vault_id TEXT NOT NULL REFERENCES vaults(id) ON DELETE CASCADE,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    PRIMARY KEY (vault_id, workspace_id)
);

CREATE INDEX IF NOT EXISTS idx_vault_workspaces_workspace ON vault_workspaces(workspace_id)
