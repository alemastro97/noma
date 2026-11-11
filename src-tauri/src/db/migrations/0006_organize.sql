-- v6: AI-written project briefs (status summary and suggested next action).
CREATE TABLE project_briefs (
  project_id    TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
  summary       TEXT,
  summary_at    TEXT,
  next_action   TEXT,
  next_item_id  TEXT REFERENCES items(id) ON DELETE SET NULL,
  next_at       TEXT
);
