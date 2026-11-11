//! Capture history (with Retry for failed captures) and personal stats.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use rusqlite::{Connection, params};
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::ai::service::AiService;
use crate::commands::{emit_items_changed, lock_message, to_message};
use crate::db::{self, DbResult};
use crate::state::{AppState, local_tz, now};

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRow {
    pub id: String,
    pub source: String,
    pub raw_text: String,
    pub status: String,
    pub latency_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub item_id: Option<String>,
    pub item_title: Option<String>,
}

/// Newest captures first, with the title of the card they produced.
pub fn recent(conn: &Connection, limit: u32) -> DbResult<Vec<CaptureRow>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.source, c.raw_text, c.status, c.latency_ms, c.created_at, c.item_id, i.title \
         FROM captures c LEFT JOIN items i ON i.id = c.item_id \
         ORDER BY c.created_at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit], |r| {
        Ok(CaptureRow {
            id: r.get(0)?,
            source: r.get(1)?,
            raw_text: r.get(2)?,
            status: r.get(3)?,
            latency_ms: r.get(4)?,
            created_at: r.get(5)?,
            item_id: r.get(6)?,
            item_title: r.get(7)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Week {
    /// Monday of the week, local date (YYYY-MM-DD).
    pub start: NaiveDate,
    pub captures: u32,
    pub completed: u32,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    /// Oldest first.
    pub weeks: Vec<Week>,
    pub avg_latency_ms: Option<f64>,
}

fn monday(day: NaiveDate) -> NaiveDate {
    day - Duration::days(day.weekday().num_days_from_monday() as i64)
}

fn times(conn: &Connection, sql: &str, since: DateTime<Utc>) -> DbResult<Vec<DateTime<Utc>>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![since], |r| r.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Captures and completions per local week over the last `weeks` weeks,
/// and the average AI latency of successful captures in that window.
pub fn stats(conn: &Connection, now: DateTime<Utc>, tz: Tz, weeks: u32) -> DbResult<Stats> {
    let this_week = monday(now.with_timezone(&tz).date_naive());
    let first = this_week - Duration::weeks(weeks.saturating_sub(1) as i64);
    let since = first
        .and_hms_opt(0, 0, 0)
        .and_then(|t| tz.from_local_datetime(&t).earliest())
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or(now - Duration::weeks(weeks as i64));
    let mut buckets: BTreeMap<NaiveDate, Week> = (0..weeks)
        .map(|i| {
            let start = first + Duration::weeks(i as i64);
            (
                start,
                Week {
                    start,
                    captures: 0,
                    completed: 0,
                },
            )
        })
        .collect();
    let week_of = |t: &DateTime<Utc>| monday(t.with_timezone(&tz).date_naive());
    for t in times(
        conn,
        "SELECT created_at FROM captures WHERE created_at >= ?1",
        since,
    )? {
        if let Some(w) = buckets.get_mut(&week_of(&t)) {
            w.captures += 1;
        }
    }
    for t in times(
        conn,
        "SELECT completed_at FROM items WHERE completed_at IS NOT NULL AND completed_at >= ?1",
        since,
    )? {
        if let Some(w) = buckets.get_mut(&week_of(&t)) {
            w.completed += 1;
        }
    }
    let avg_latency_ms = conn.query_row(
        "SELECT AVG(latency_ms) FROM captures \
         WHERE status = 'done' AND latency_ms IS NOT NULL AND created_at >= ?1",
        params![since],
        |r| r.get(0),
    )?;
    Ok(Stats {
        weeks: buckets.into_values().collect(),
        avg_latency_ms,
    })
}

#[tauri::command]
pub async fn captures_recent(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<CaptureRow>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    recent(&conn, limit.unwrap_or(50).min(500)).map_err(to_message)
}

/// Puts one failed capture back in the queue.
#[tauri::command]
pub async fn captures_retry(
    app: AppHandle,
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
    id: String,
) -> Result<(), String> {
    let item = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::captures::retry(&conn, &id).map_err(to_message)?;
        db::captures::get_capture(&conn, &id)
            .map_err(to_message)?
            .and_then(|c| c.item_id)
    };
    ai.nudge();
    ai.publish().await;
    emit_items_changed(&app, item.into_iter().collect());
    Ok(())
}

#[tauri::command]
pub async fn stats_get(state: State<'_, AppState>) -> Result<Stats, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    stats(&conn, now(), local_tz(), 8).map_err(to_message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::captures::{CaptureStatus, create_capture, set_status};
    use crate::db::models::NewItem;

    #[test]
    fn history_and_weekly_stats() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 11, 12, 0, 0).unwrap();
        let item = db::items::create_item(
            &conn,
            NewItem {
                title: "buy milk".into(),
                ..Default::default()
            },
            now,
        )
        .unwrap();
        let a = create_capture(&conn, "text", "buy milk", Some(&item.id), now).unwrap();
        set_status(&conn, &a, CaptureStatus::Done, Some("{}"), Some(800)).unwrap();
        let old = now - Duration::weeks(2);
        let b = create_capture(&conn, "voice", "call mum", None, old).unwrap();
        set_status(&conn, &b, CaptureStatus::Done, Some("{}"), Some(1200)).unwrap();
        db::items::complete_item(&conn, &item.id, now, chrono_tz::UTC).unwrap();

        let rows = recent(&conn, 10).unwrap();
        assert_eq!(rows[0].id, a);
        assert_eq!(rows[0].item_title.as_deref(), Some("buy milk"));

        let s = stats(&conn, now, chrono_tz::UTC, 4).unwrap();
        assert_eq!(s.weeks.len(), 4);
        let last = s.weeks.last().unwrap();
        assert_eq!(last.start, NaiveDate::from_ymd_opt(2026, 11, 9).unwrap());
        assert_eq!((last.captures, last.completed), (1, 1));
        assert_eq!(s.weeks[1].captures, 1);
        assert_eq!(s.avg_latency_ms, Some(1000.0));
    }
}
