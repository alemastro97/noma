-- NOMA schema v1. Timestamps are written by rusqlite's chrono ToSql as
-- "YYYY-MM-DD HH:MM:SS[.f]+00:00" (UTC); always bind through ToSql, never compare
-- against hand-written literals. Ids are UUID v7 text.

CREATE TABLE IF NOT EXISTS schema_version (
  version INTEGER NOT NULL
);

CREATE TABLE boards (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  kind        TEXT NOT NULL CHECK (kind IN ('project', 'personal')),
  created_at  TEXT NOT NULL
);

CREATE TABLE projects (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  color       TEXT NOT NULL,
  board_id    TEXT NOT NULL REFERENCES boards(id) ON DELETE RESTRICT,
  created_at  TEXT NOT NULL
);

CREATE TABLE items (
  id             TEXT PRIMARY KEY,
  kind           TEXT NOT NULL CHECK (kind IN ('task', 'reminder', 'note', 'idea', 'project_ref', 'event')),
  title          TEXT NOT NULL,
  body           TEXT,
  status         TEXT NOT NULL DEFAULT 'inbox' CHECK (status IN ('inbox', 'next', 'doing', 'done')),
  priority       TEXT NOT NULL DEFAULT 'normal' CHECK (priority IN ('low', 'normal', 'high', 'urgent')),
  due_at         TEXT,
  remind_at      TEXT,
  snoozed_until  TEXT,
  recurrence     TEXT,
  project_id     TEXT REFERENCES projects(id) ON DELETE SET NULL,
  created_at     TEXT NOT NULL,
  updated_at     TEXT NOT NULL,
  completed_at   TEXT
);

CREATE INDEX idx_items_status     ON items(status);
CREATE INDEX idx_items_due_at     ON items(due_at);
CREATE INDEX idx_items_remind_at  ON items(remind_at);
CREATE INDEX idx_items_project_id ON items(project_id);
CREATE INDEX idx_items_kind       ON items(kind);

CREATE TABLE placements (
  item_id   TEXT PRIMARY KEY REFERENCES items(id) ON DELETE CASCADE,
  board_id  TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
  x         REAL NOT NULL DEFAULT 0,
  y         REAL NOT NULL DEFAULT 0
);

CREATE INDEX idx_placements_board ON placements(board_id);

CREATE TABLE connections (
  id         TEXT PRIMARY KEY,
  from_item  TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  to_item    TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  label      TEXT,
  created_at TEXT NOT NULL,
  UNIQUE (from_item, to_item)
);

CREATE TABLE tags (
  id    TEXT PRIMARY KEY,
  name  TEXT NOT NULL UNIQUE COLLATE NOCASE
);

CREATE TABLE item_tags (
  item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  tag_id  TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (item_id, tag_id)
);

CREATE TABLE captures (
  id                 TEXT PRIMARY KEY,
  source             TEXT NOT NULL CHECK (source IN ('voice', 'text')),
  raw_text           TEXT NOT NULL,
  model_output_json  TEXT,
  status             TEXT NOT NULL CHECK (status IN ('pending', 'processing', 'done', 'failed')),
  latency_ms         INTEGER,
  item_id            TEXT REFERENCES items(id) ON DELETE SET NULL,
  created_at         TEXT NOT NULL
);

CREATE TABLE item_occurrences (
  id            TEXT PRIMARY KEY,
  item_id       TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  scheduled_at  TEXT NOT NULL,
  completed_at  TEXT,
  skipped       INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_occurrences_item ON item_occurrences(item_id);

CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

-- Full-text search over items (external content table).
CREATE VIRTUAL TABLE items_fts USING fts5(
  title,
  body,
  content = 'items',
  content_rowid = 'rowid'
);

CREATE TRIGGER items_ai AFTER INSERT ON items BEGIN
  INSERT INTO items_fts(rowid, title, body) VALUES (new.rowid, new.title, new.body);
END;

CREATE TRIGGER items_ad AFTER DELETE ON items BEGIN
  INSERT INTO items_fts(items_fts, rowid, title, body) VALUES ('delete', old.rowid, old.title, old.body);
END;

CREATE TRIGGER items_au AFTER UPDATE OF title, body ON items BEGIN
  INSERT INTO items_fts(items_fts, rowid, title, body) VALUES ('delete', old.rowid, old.title, old.body);
  INSERT INTO items_fts(rowid, title, body) VALUES (new.rowid, new.title, new.body);
END;
