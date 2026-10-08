-- Runs are written as they go, and a run stays unsaved until the user names it.
ALTER TABLE fuzz_runs ADD COLUMN saved INTEGER NOT NULL DEFAULT 1;
ALTER TABLE fuzz_results ADD COLUMN matches_json TEXT NOT NULL DEFAULT '[]';
