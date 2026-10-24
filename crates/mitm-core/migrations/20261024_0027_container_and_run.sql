-- Firefork containers: which container sent a flow, and the run boundary within it.
ALTER TABLE history ADD COLUMN container_label TEXT NOT NULL DEFAULT '';
ALTER TABLE history ADD COLUMN run_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE ws_connections ADD COLUMN container_label TEXT NOT NULL DEFAULT '';
ALTER TABLE ws_connections ADD COLUMN run_id INTEGER NOT NULL DEFAULT 0;
CREATE INDEX idx_history_container_run ON history(container_label, run_id);
CREATE INDEX idx_ws_connections_container_run ON ws_connections(container_label, run_id);
