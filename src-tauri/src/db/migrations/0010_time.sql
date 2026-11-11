-- Time features: extra reminders before a due time, habits and focus sessions.

-- "Remind me 1 day / 1 hour before": minutes before `items.due_at`.
-- `fired_for` is the due time the offset last fired for, so moving the due
-- date makes it fire again.
CREATE TABLE reminder_offsets (
  item_id    TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  minutes    INTEGER NOT NULL CHECK (minutes > 0),
  fired_for  TEXT,
  PRIMARY KEY (item_id, minutes)
);

-- Recurring items the user tracks as habits (streak + 7-day strip).
CREATE TABLE habits (
  item_id     TEXT PRIMARY KEY REFERENCES items(id) ON DELETE CASCADE,
  created_at  TEXT NOT NULL
);

-- Focus timer sessions, logged per card.
CREATE TABLE focus_sessions (
  id          TEXT PRIMARY KEY,
  item_id     TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  started_at  TEXT NOT NULL,
  ended_at    TEXT NOT NULL,
  minutes     INTEGER NOT NULL,
  completed   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_focus_sessions_item ON focus_sessions(item_id);
