-- DB Viewer connections and saved queries. `data` is the JSON the viewer serialises, and the key columns are copies for lookup.
-- Existing ~/.local/share/dbviewer/*.json files are imported in Rust (db::dbviewer_db::import_json_files).
CREATE TABLE IF NOT EXISTS dbviewer_connections (
    name TEXT PRIMARY KEY,
    position INTEGER NOT NULL,
    data TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS dbviewer_queries (
    connection TEXT NOT NULL,
    name TEXT NOT NULL,
    position INTEGER NOT NULL,
    data TEXT NOT NULL,
    PRIMARY KEY (connection, name)
)
