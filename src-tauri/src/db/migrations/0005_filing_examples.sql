-- Where the user moved a capture after NOMA filed it: few-shot examples for filing.
CREATE TABLE filing_examples (
  id          TEXT PRIMARY KEY,
  text        TEXT NOT NULL UNIQUE,
  place_kind  TEXT NOT NULL CHECK (place_kind IN ('project', 'board', 'inbox')),
  place_id    TEXT,
  created_at  TEXT NOT NULL
);
