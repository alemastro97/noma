-- v2: event end time, fixed recurrence series start, and an FTS index keyed by
-- item id (the v1 index used the implicit rowid of a TEXT-keyed table, which
-- VACUUM may renumber).

ALTER TABLE items ADD COLUMN ends_at TEXT;
ALTER TABLE items ADD COLUMN recurrence_start TEXT;
UPDATE items SET recurrence_start = COALESCE(due_at, remind_at) WHERE recurrence IS NOT NULL;

CREATE INDEX idx_connections_from ON connections(from_item);
CREATE INDEX idx_connections_to   ON connections(to_item);

DROP TRIGGER IF EXISTS items_ai;
DROP TRIGGER IF EXISTS items_ad;
DROP TRIGGER IF EXISTS items_au;
DROP TABLE IF EXISTS items_fts;

CREATE VIRTUAL TABLE items_fts USING fts5(id UNINDEXED, title, body);
INSERT INTO items_fts (id, title, body) SELECT id, title, COALESCE(body, '') FROM items;

CREATE TRIGGER items_ai AFTER INSERT ON items BEGIN
  INSERT INTO items_fts (id, title, body) VALUES (new.id, new.title, COALESCE(new.body, ''));
END;

CREATE TRIGGER items_ad AFTER DELETE ON items BEGIN
  DELETE FROM items_fts WHERE id = old.id;
END;

CREATE TRIGGER items_au AFTER UPDATE OF title, body ON items BEGIN
  UPDATE items_fts SET title = new.title, body = COALESCE(new.body, '') WHERE id = new.id;
END;
