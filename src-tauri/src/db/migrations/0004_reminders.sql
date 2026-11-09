-- v4: when a reminder last fired, so it fires once per remind/snooze time.
ALTER TABLE items ADD COLUMN reminded_at TEXT;
