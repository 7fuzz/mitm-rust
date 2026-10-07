-- Safe migration: Add action column if missing
ALTER TABLE intercept_rules ADD COLUMN action TEXT NOT NULL DEFAULT 'intercept';
