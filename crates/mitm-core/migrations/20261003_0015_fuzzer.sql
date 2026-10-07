-- Saved fuzzer attacks and their results

CREATE TABLE IF NOT EXISTS fuzz_runs (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    attack_type   TEXT NOT NULL,
    config_json   TEXT NOT NULL,     -- request template, positions, payload sets, options
    total         INTEGER NOT NULL DEFAULT 0,
    created_at_ms  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS fuzz_results (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id               TEXT NOT NULL,
    idx                  INTEGER NOT NULL,     -- 0-based position in the attack
    payloads_json        TEXT NOT NULL,        -- the payload value per position
    method               TEXT NOT NULL,
    url                  TEXT NOT NULL,
    request_headers_json TEXT NOT NULL,
    request_body         TEXT,
    status_code          INTEGER NOT NULL,
    response_headers_json TEXT NOT NULL,
    response_body        TEXT,
    response_size        INTEGER NOT NULL DEFAULT 0,
    duration_ms          INTEGER NOT NULL DEFAULT 0,
    error                TEXT,
    FOREIGN KEY (run_id) REFERENCES fuzz_runs(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_fuzz_results_run ON fuzz_results(run_id, idx);
