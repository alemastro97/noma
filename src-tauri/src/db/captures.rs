//! Every AI capture is a row: the raw text, what the model returned, how long
//! it took and which card it produced. Pending captures survive restarts.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;

use super::error::DbResult;
use super::models::new_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureStatus {
    Pending,
    Processing,
    Done,
    Failed,
}

impl CaptureStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capture {
    pub id: String,
    pub source: String,
    pub raw_text: String,
    pub status: String,
    pub item_id: Option<String>,
    pub latency_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
}

fn row_to_capture(r: &Row<'_>) -> rusqlite::Result<Capture> {
    Ok(Capture {
        id: r.get("id")?,
        source: r.get("source")?,
        raw_text: r.get("raw_text")?,
        status: r.get("status")?,
        item_id: r.get("item_id")?,
        latency_ms: r.get("latency_ms")?,
        created_at: r.get("created_at")?,
    })
}

pub fn create_capture(
    conn: &Connection,
    source: &str,
    raw_text: &str,
    item_id: Option<&str>,
    now: DateTime<Utc>,
) -> DbResult<String> {
    let id = new_id();
    conn.execute(
        "INSERT INTO captures (id, source, raw_text, status, item_id, created_at) \
         VALUES (?1, ?2, ?3, 'pending', ?4, ?5)",
        params![id, source, raw_text, item_id, now],
    )?;
    Ok(id)
}

pub fn get_capture(conn: &Connection, id: &str) -> DbResult<Option<Capture>> {
    Ok(conn
        .query_row("SELECT * FROM captures WHERE id = ?1", [id], row_to_capture)
        .optional()?)
}

/// Oldest capture still waiting for the model.
pub fn next_pending(conn: &Connection) -> DbResult<Option<Capture>> {
    Ok(conn
        .query_row(
            "SELECT * FROM captures WHERE status = 'pending' ORDER BY created_at LIMIT 1",
            [],
            row_to_capture,
        )
        .optional()?)
}

pub fn count_pending(conn: &Connection) -> DbResult<u32> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM captures WHERE status IN ('pending', 'processing')",
        [],
        |r| r.get(0),
    )?)
}

pub fn set_status(
    conn: &Connection,
    id: &str,
    status: CaptureStatus,
    model_output: Option<&str>,
    latency_ms: Option<i64>,
) -> DbResult<()> {
    conn.execute(
        "UPDATE captures SET status = ?2, model_output_json = COALESCE(?3, model_output_json), \
         latency_ms = COALESCE(?4, latency_ms) WHERE id = ?1",
        params![id, status.as_str(), model_output, latency_ms],
    )?;
    Ok(())
}

/// Captures interrupted mid-flight (app quit while processing) go back to pending.
pub fn requeue_interrupted(conn: &Connection) -> DbResult<usize> {
    Ok(conn.execute(
        "UPDATE captures SET status = 'pending' WHERE status = 'processing'",
        [],
    )?)
}

/// Retrying a failed capture: back to pending.
pub fn retry(conn: &Connection, id: &str) -> DbResult<()> {
    conn.execute(
        "UPDATE captures SET status = 'pending' WHERE id = ?1 AND status = 'failed'",
        [id],
    )?;
    Ok(())
}

pub fn latest_for_item(conn: &Connection, item_id: &str) -> DbResult<Option<Capture>> {
    Ok(conn
        .query_row(
            "SELECT * FROM captures WHERE item_id = ?1 ORDER BY created_at DESC LIMIT 1",
            [item_id],
            row_to_capture,
        )
        .optional()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn lifecycle_and_capturing_flag() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let item = db::items::create_item(
            &conn,
            db::models::NewItem {
                title: "buy milk".into(),
                ..Default::default()
            },
            now,
        )
        .unwrap();
        let id = create_capture(&conn, "text", "buy milk", Some(&item.id), now).unwrap();
        assert!(
            db::items::get_item(&conn, &item.id)
                .unwrap()
                .unwrap()
                .capturing
        );
        assert_eq!(next_pending(&conn).unwrap().unwrap().id, id);
        set_status(&conn, &id, CaptureStatus::Processing, None, None).unwrap();
        assert!(next_pending(&conn).unwrap().is_none());
        assert_eq!(requeue_interrupted(&conn).unwrap(), 1);
        set_status(&conn, &id, CaptureStatus::Failed, Some("{}"), Some(900)).unwrap();
        assert!(
            !db::items::get_item(&conn, &item.id)
                .unwrap()
                .unwrap()
                .capturing
        );
        retry(&conn, &id).unwrap();
        assert_eq!(get_capture(&conn, &id).unwrap().unwrap().status, "pending");
        assert_eq!(latest_for_item(&conn, &item.id).unwrap().unwrap().id, id);
        assert_eq!(count_pending(&conn).unwrap(), 1);
    }
}
