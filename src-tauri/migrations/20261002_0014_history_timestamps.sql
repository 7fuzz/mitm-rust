-- When the proxy received the request and when the exchange finished (RFC3339, ms precision)
ALTER TABLE history ADD COLUMN request_at TEXT;
ALTER TABLE history ADD COLUMN response_at TEXT;
