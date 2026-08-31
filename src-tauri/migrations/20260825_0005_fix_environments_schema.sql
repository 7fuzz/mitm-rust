-- Migration 0005: Ensure environments and collections tables have all workspace columns if upgraded from legacy schema

ALTER TABLE environments ADD COLUMN workspace_id TEXT NOT NULL DEFAULT '';
ALTER TABLE environments ADD COLUMN variables_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE environments ADD COLUMN created_at_ms INTEGER NOT NULL DEFAULT 0;
ALTER TABLE environments ADD COLUMN updated_at_ms INTEGER NOT NULL DEFAULT 0;

ALTER TABLE collections ADD COLUMN workspace_id TEXT NOT NULL DEFAULT '';
ALTER TABLE collections ADD COLUMN parent_id TEXT;
ALTER TABLE collections ADD COLUMN description TEXT;
ALTER TABLE collections ADD COLUMN order_index INTEGER NOT NULL DEFAULT 0;
ALTER TABLE collections ADD COLUMN created_at_ms INTEGER NOT NULL DEFAULT 0;
ALTER TABLE collections ADD COLUMN updated_at_ms INTEGER NOT NULL DEFAULT 0;
