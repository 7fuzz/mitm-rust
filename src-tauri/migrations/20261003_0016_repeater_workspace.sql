INSERT OR IGNORE INTO workspaces (id, name, description, active_environment_id, created_at_ms, updated_at_ms)
VALUES ('00000000-0000-4000-8000-000000000001', 'Repeater', 'Requests from the Repeater', NULL, 0, 0);

INSERT OR IGNORE INTO collections (id, workspace_id, parent_id, name, description, order_index, created_at_ms, updated_at_ms)
VALUES ('00000000-0000-4000-8000-000000000002', '00000000-0000-4000-8000-000000000001', NULL, 'Uncategorized', NULL, 0, 0, 0);

INSERT OR IGNORE INTO requests (id, collection_id, name, method, url, headers_json, params_json, body_type, body_json, body_raw, body_form_data, body_urlencoded, extract_rules_json, description, order_index, created_at_ms, updated_at_ms)
SELECT
    id,
    '00000000-0000-4000-8000-000000000002',
    method || ' ' || url,
    method,
    url,
    headers_json,
    params_json,
    body_type,
    CASE WHEN body_type = 'json' THEN body_content END,
    CASE WHEN body_type NOT IN ('json', 'form', 'form-data', 'multipart', 'urlencoded', 'x-www-form-urlencoded') THEN body_content END,
    CASE WHEN body_type IN ('form', 'form-data', 'multipart') THEN body_content END,
    CASE WHEN body_type IN ('urlencoded', 'x-www-form-urlencoded') THEN body_content END,
    extract_rules_json,
    NULL,
    ROW_NUMBER() OVER (ORDER BY created_at_ms DESC, updated_at_ms DESC) - 1,
    created_at_ms,
    updated_at_ms
FROM repeaters;

INSERT INTO request_histories (request_id, method, url, request_headers_json, request_body, status_code, response_headers_json, response_body, duration_ms, executed_at_ms)
SELECT repeater_id, method, url, request_headers_json, request_body, status_code, response_headers_json, response_body, duration_ms, executed_at_ms
FROM repeater_histories
WHERE repeater_id IN (SELECT id FROM requests)
ORDER BY id;

DROP TABLE IF EXISTS repeater_histories;

DROP TABLE IF EXISTS repeaters;
