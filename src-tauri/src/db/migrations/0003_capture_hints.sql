-- v3: review hints left by AI capture ("check date", "new project?", low confidence).
ALTER TABLE items ADD COLUMN hint TEXT;
CREATE INDEX idx_captures_status ON captures(status, created_at);
CREATE INDEX idx_captures_item ON captures(item_id);
