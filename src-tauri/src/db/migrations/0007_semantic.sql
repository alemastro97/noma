-- Semantic index: one embedding per card (title + body). See docs/specs/semantic.md.
-- `content_hash` is the SHA-256 of the embedded text + model id, so a card is
-- re-embedded only when its text (or the embedding model) changes.
CREATE TABLE embeddings (
  item_id       TEXT PRIMARY KEY REFERENCES items(id) ON DELETE CASCADE,
  model_id      TEXT NOT NULL,
  content_hash  TEXT NOT NULL,
  dims          INTEGER NOT NULL,
  vector        BLOB NOT NULL,
  updated_at    TEXT NOT NULL
);
