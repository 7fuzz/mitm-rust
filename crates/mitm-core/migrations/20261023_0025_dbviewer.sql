-- DB Viewer saved queries, one JSON document per row. The key columns are copies for lookup.
-- Existing ~/.local/share/dbviewer/queries.json is imported in Rust (db::dbviewer_db::import_json_files).
CREATE TABLE IF NOT EXISTS dbviewer_queries (
    connection TEXT NOT NULL,
    name TEXT NOT NULL,
    position INTEGER NOT NULL,
    data TEXT NOT NULL,
    PRIMARY KEY (connection, name)
)
