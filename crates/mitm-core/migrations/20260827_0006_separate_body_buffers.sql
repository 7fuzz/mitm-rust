ALTER TABLE requests ADD COLUMN body_json TEXT;
ALTER TABLE requests ADD COLUMN body_raw TEXT;
ALTER TABLE requests ADD COLUMN body_form_data TEXT;
ALTER TABLE requests ADD COLUMN body_urlencoded TEXT;

UPDATE requests SET body_raw = body_content WHERE body_raw IS NULL AND body_content IS NOT NULL;
UPDATE requests SET body_json = body_content WHERE body_type = 'json' AND body_json IS NULL;
UPDATE requests SET body_form_data = body_content WHERE (body_type = 'form-data' OR body_type = 'multipart') AND body_form_data IS NULL;
UPDATE requests SET body_urlencoded = body_content WHERE (body_type = 'urlencoded' OR body_type = 'x-www-form-urlencoded') AND body_urlencoded IS NULL;
