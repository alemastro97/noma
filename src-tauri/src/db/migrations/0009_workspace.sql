-- v9 (workspace): saved filters shown in the sidebar, and labelled group frames
-- drawn on a board's canvas.
CREATE TABLE saved_filters (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  spec        TEXT NOT NULL,
  created_at  TEXT NOT NULL
);

CREATE TABLE canvas_groups (
  id          TEXT PRIMARY KEY,
  board_id    TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
  label       TEXT NOT NULL,
  x           REAL NOT NULL,
  y           REAL NOT NULL,
  w           REAL NOT NULL,
  h           REAL NOT NULL,
  created_at  TEXT NOT NULL
);

CREATE INDEX idx_canvas_groups_board ON canvas_groups(board_id);
