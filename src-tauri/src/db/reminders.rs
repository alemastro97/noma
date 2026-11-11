//! Which reminders are due. A reminder fires at `COALESCE(snoozed_until,
//! remind_at)` once: `reminded_at` records the moment it fired, so snoozing
//! (a later effective time) or editing the time makes it fire again.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};

use super::error::DbResult;
use super::items::get_item;
use super::models::Item;

const EFFECTIVE: &str = "COALESCE(snoozed_until, remind_at)";

/// Open items whose reminder time has come and has not fired yet.
pub fn due_reminders(conn: &Connection, now: DateTime<Utc>) -> DbResult<Vec<Item>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT id FROM items WHERE status != 'done' AND {EFFECTIVE} IS NOT NULL \
         AND archived_at IS NULL AND deleted_at IS NULL \
         AND {EFFECTIVE} <= ?1 AND (reminded_at IS NULL OR reminded_at < {EFFECTIVE}) \
         ORDER BY {EFFECTIVE}"
    ))?;
    let ids: Vec<String> = stmt
        .query_map([now], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut items = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(item) = get_item(conn, &id)? {
            items.push(item);
        }
    }
    Ok(items)
}

/// Records that `item_id`'s reminder fired at its current effective time.
pub fn mark_reminded(conn: &Connection, item_id: &str) -> DbResult<()> {
    conn.execute(
        &format!("UPDATE items SET reminded_at = {EFFECTIVE} WHERE id = ?1"),
        params![item_id],
    )?;
    Ok(())
}

/// The next moment a reminder will be due, if any.
pub fn next_reminder_at(conn: &Connection, now: DateTime<Utc>) -> DbResult<Option<DateTime<Utc>>> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT MIN({EFFECTIVE}) FROM items WHERE status != 'done' \
                 AND archived_at IS NULL AND deleted_at IS NULL \
                 AND {EFFECTIVE} > ?1 AND (reminded_at IS NULL OR reminded_at < {EFFECTIVE})"
            ),
            [now],
            |r| r.get::<_, Option<DateTime<Utc>>>(0),
        )
        .optional()?
        .flatten())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{
        self,
        models::{ItemPatch, ItemType, NewItem},
    };
    use chrono::{Duration, TimeZone};

    fn utc(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 9, h, m, 0).unwrap()
    }

    #[test]
    fn fires_once_then_again_after_snooze() {
        let conn = db::open_in_memory().unwrap();
        let item = db::items::create_item(
            &conn,
            NewItem {
                kind: ItemType::Reminder,
                title: "Call".into(),
                remind_at: Some(utc(16, 0)),
                ..Default::default()
            },
            utc(9, 0),
        )
        .unwrap();
        assert!(due_reminders(&conn, utc(15, 59)).unwrap().is_empty());
        assert_eq!(
            next_reminder_at(&conn, utc(15, 0)).unwrap(),
            Some(utc(16, 0))
        );
        assert_eq!(due_reminders(&conn, utc(16, 0)).unwrap().len(), 1);
        mark_reminded(&conn, &item.id).unwrap();
        assert!(due_reminders(&conn, utc(16, 5)).unwrap().is_empty());
        assert_eq!(next_reminder_at(&conn, utc(16, 5)).unwrap(), None);

        let snooze = ItemPatch {
            snoozed_until: Some(Some(utc(17, 0))),
            ..Default::default()
        };
        db::items::update_item(&conn, &item.id, snooze, utc(16, 5), chrono_tz::UTC).unwrap();
        assert!(due_reminders(&conn, utc(16, 30)).unwrap().is_empty());
        assert_eq!(
            due_reminders(&conn, utc(17, 0) + Duration::seconds(1))
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn done_items_never_fire() {
        let conn = db::open_in_memory().unwrap();
        let item = db::items::create_item(
            &conn,
            NewItem {
                kind: ItemType::Reminder,
                title: "Done already".into(),
                remind_at: Some(utc(10, 0)),
                ..Default::default()
            },
            utc(9, 0),
        )
        .unwrap();
        db::items::complete_item(&conn, &item.id, utc(9, 30), chrono_tz::UTC).unwrap();
        assert!(due_reminders(&conn, utc(11, 0)).unwrap().is_empty());
    }
}
