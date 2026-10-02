-- Add listener_label column to history table for multi-listener support
ALTER TABLE history ADD COLUMN listener_label TEXT NOT NULL DEFAULT '';
