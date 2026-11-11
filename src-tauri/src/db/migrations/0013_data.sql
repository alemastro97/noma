-- Capture history and personal stats read captures and completions by date.
CREATE INDEX IF NOT EXISTS idx_captures_created_at ON captures(created_at);
CREATE INDEX IF NOT EXISTS idx_items_completed_at ON items(completed_at);
