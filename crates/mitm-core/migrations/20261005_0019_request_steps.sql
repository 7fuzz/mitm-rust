CREATE TABLE IF NOT EXISTS request_steps (
    request_id TEXT NOT NULL REFERENCES requests(id) ON DELETE CASCADE,
    phase TEXT NOT NULL CHECK (phase IN ('pre', 'post')),
    position INTEGER NOT NULL,
    step_request_id TEXT NOT NULL REFERENCES requests(id) ON DELETE CASCADE,
    PRIMARY KEY (request_id, phase, position),
    CHECK (step_request_id != request_id)
);

CREATE INDEX IF NOT EXISTS idx_request_steps_step ON request_steps(step_request_id);
