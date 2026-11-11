//! Original audio of voice captures (opt-in, `voice.keepAudio`): one WAV file
//! per capture in `<app data>/recordings`, linked to the capture row.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;

use super::error::DbResult;
use super::models::new_id;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Recording {
    pub id: String,
    pub capture_id: String,
    /// File name inside the recordings directory.
    #[serde(skip)]
    pub file: String,
    pub duration_ms: i64,
    pub created_at: DateTime<Utc>,
}

fn row_to_recording(r: &Row<'_>) -> rusqlite::Result<Recording> {
    Ok(Recording {
        id: r.get("id")?,
        capture_id: r.get("capture_id")?,
        file: r.get("file")?,
        duration_ms: r.get("duration_ms")?,
        created_at: r.get("created_at")?,
    })
}

/// Records that `file` holds the audio of `capture_id`.
pub fn insert_recording(
    conn: &Connection,
    id: &str,
    capture_id: &str,
    file: &str,
    duration_ms: i64,
    now: DateTime<Utc>,
) -> DbResult<()> {
    conn.execute(
        "INSERT INTO voice_recordings (id, capture_id, file, duration_ms, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, capture_id, file, duration_ms, now],
    )?;
    Ok(())
}

/// A fresh id for a recording (also its file stem).
pub fn new_recording_id() -> String {
    new_id()
}

pub fn get_recording(conn: &Connection, id: &str) -> DbResult<Option<Recording>> {
    Ok(conn
        .query_row(
            "SELECT * FROM voice_recordings WHERE id = ?1",
            [id],
            row_to_recording,
        )
        .optional()?)
}

/// The latest recording of a capture that produced (or was turned into) this card.
pub fn recording_for_item(conn: &Connection, item_id: &str) -> DbResult<Option<Recording>> {
    Ok(conn
        .query_row(
            "SELECT r.* FROM voice_recordings r JOIN captures c ON c.id = r.capture_id \
             WHERE c.item_id = ?1 ORDER BY r.created_at DESC LIMIT 1",
            [item_id],
            row_to_recording,
        )
        .optional()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn recording_is_found_through_its_capture() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let item = db::items::create_item(
            &conn,
            db::models::NewItem {
                title: "call Marta".into(),
                ..Default::default()
            },
            now,
        )
        .unwrap();
        let capture =
            db::captures::create_capture(&conn, "voice", "call Marta", Some(&item.id), now)
                .unwrap();
        assert!(recording_for_item(&conn, &item.id).unwrap().is_none());
        insert_recording(&conn, "r1", &capture, "r1.wav", 1800, now).unwrap();
        let r = recording_for_item(&conn, &item.id).unwrap().unwrap();
        assert_eq!((r.id.as_str(), r.duration_ms), ("r1", 1800));
        assert_eq!(get_recording(&conn, "r1").unwrap().unwrap().file, "r1.wav");
    }
}
