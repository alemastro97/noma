-- The FTS row was rewritten (a full scan of the index: its id column is not
-- indexed) on every card update, because every write lists title and body.
-- Only do it when the text really changed.
DROP TRIGGER IF EXISTS items_au;
CREATE TRIGGER items_au AFTER UPDATE OF title, body ON items
WHEN old.title IS NOT new.title OR old.body IS NOT new.body
BEGIN
  UPDATE items_fts SET title = new.title, body = COALESCE(new.body, '') WHERE id = new.id;
END;
