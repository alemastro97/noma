-- Cards: pinning, archive and soft delete (Trash) on items; ordered subtasks;
-- file attachments; archived projects and boards.
ALTER TABLE items ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
ALTER TABLE items ADD COLUMN archived_at TEXT;
ALTER TABLE items ADD COLUMN deleted_at TEXT;
CREATE INDEX idx_items_archived_at ON items(archived_at);
CREATE INDEX idx_items_deleted_at ON items(deleted_at);

ALTER TABLE projects ADD COLUMN archived_at TEXT;
ALTER TABLE boards ADD COLUMN archived_at TEXT;

CREATE TABLE subtasks (
  id          TEXT PRIMARY KEY,
  item_id     TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  title       TEXT NOT NULL,
  done        INTEGER NOT NULL DEFAULT 0,
  position    INTEGER NOT NULL,
  created_at  TEXT NOT NULL
);
CREATE INDEX idx_subtasks_item ON subtasks(item_id, position);

-- `file` is relative to <app data>/attachments.
CREATE TABLE attachments (
  id          TEXT PRIMARY KEY,
  item_id     TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  name        TEXT NOT NULL,
  file        TEXT NOT NULL,
  mime        TEXT NOT NULL,
  size        INTEGER NOT NULL,
  created_at  TEXT NOT NULL
);
CREATE INDEX idx_attachments_item ON attachments(item_id);
