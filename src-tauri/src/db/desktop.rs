//! Read-only queries behind the desktop presence: the menu-bar count, the orb's
//! attention glow and "open last capture".

use chrono::{DateTime, Duration, Utc};
use chrono_tz::Tz;
use rusqlite::{Connection, OptionalExtension};

use super::error::DbResult;
use super::items::list_items;
use super::models::ItemFilter;

/// The orb glows when a reminder is due within this window…
pub const ATTENTION_AHEAD: Duration = Duration::minutes(15);
/// …or became overdue at most this long ago (older ones would glow forever).
pub const ATTENTION_BEHIND: Duration = Duration::hours(24);

/// Open items on Today (the same list as the orb panel's "n left").
pub fn today_left(conn: &Connection, now: DateTime<Utc>, tz: Tz) -> DbResult<u32> {
    Ok(list_items(conn, &ItemFilter::Today { now, tz }, false)?.len() as u32)
}

/// Open items whose reminder time (snooze first) is overdue or due soon.
pub fn attention_count(conn: &Connection, now: DateTime<Utc>) -> DbResult<u32> {
    Ok(conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM items i WHERE status != 'done' AND {} \
             AND COALESCE(snoozed_until, remind_at) IS NOT NULL \
             AND COALESCE(snoozed_until, remind_at) <= ?1 \
             AND COALESCE(snoozed_until, remind_at) >= ?2",
            super::items::VISIBLE
        ),
        rusqlite::params![now + ATTENTION_AHEAD, now - ATTENTION_BEHIND],
        |r| r.get(0),
    )?)
}

/// The card produced by the most recent capture that still exists.
pub fn last_capture_item(conn: &Connection) -> DbResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT c.item_id FROM captures c JOIN items i ON i.id = c.item_id \
             ORDER BY c.created_at DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{
        self,
        models::{ItemType, NewItem},
    };
    use chrono::TimeZone;

    fn reminder(conn: &Connection, at: DateTime<Utc>) {
        db::items::create_item(
            conn,
            NewItem {
                kind: ItemType::Reminder,
                title: "Call".into(),
                remind_at: Some(at),
                ..Default::default()
            },
            at - Duration::days(2),
        )
        .unwrap();
    }

    #[test]
    fn attention_covers_soon_and_recently_overdue() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 11, 10, 0, 0).unwrap();
        reminder(&conn, now + Duration::hours(2));
        assert_eq!(attention_count(&conn, now).unwrap(), 0);
        reminder(&conn, now + Duration::minutes(10));
        reminder(&conn, now - Duration::hours(1));
        reminder(&conn, now - Duration::days(3));
        assert_eq!(attention_count(&conn, now).unwrap(), 2);
    }

    #[test]
    fn no_capture_no_item() {
        let conn = db::open_in_memory().unwrap();
        assert_eq!(last_capture_item(&conn).unwrap(), None);
    }
}
