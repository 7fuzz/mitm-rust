-- Free-text tags, plaintext like names so they show and filter while credentials are locked.
ALTER TABLE credentials ADD COLUMN tags_json TEXT NOT NULL DEFAULT '[]'
