-- Original audio of voice captures (opt-in): one WAV file per capture in app data.
CREATE TABLE voice_recordings (
  id           TEXT PRIMARY KEY,
  capture_id   TEXT NOT NULL REFERENCES captures(id) ON DELETE CASCADE,
  file         TEXT NOT NULL,
  duration_ms  INTEGER NOT NULL,
  created_at   TEXT NOT NULL
);
CREATE INDEX idx_voice_recordings_capture ON voice_recordings(capture_id);
