//! Time features: extra reminders before a due time ("offsets"), habits
//! (streak + 7-day strip computed from `item_occurrences`) and the focus-timer
//! session log. Pure functions over a connection; the clock is passed in.

use std::collections::HashSet;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use chrono_tz::Tz;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use super::error::{DbError, DbResult};
use super::items::{get_item, local_day_bounds};
use super::models::{Item, Status, new_id};

/// Longest offset accepted: 30 days before.
pub const MAX_OFFSET_MINUTES: i64 = 60 * 24 * 30;

fn require(conn: &Connection, id: &str) -> DbResult<Item> {
    get_item(conn, id)?.ok_or_else(|| DbError::NotFound(format!("item {id}")))
}

// ── Reminder offsets ────────────────────────────────────────────────────

/// Offsets (minutes before the due time) for an item, smallest first.
pub fn get_offsets(conn: &Connection, item_id: &str) -> DbResult<Vec<i64>> {
    let mut stmt =
        conn.prepare("SELECT minutes FROM reminder_offsets WHERE item_id = ?1 ORDER BY minutes")?;
    let rows = stmt.query_map([item_id], |r| r.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Replaces an item's offsets. Kept offsets remember when they last fired.
pub fn set_offsets(conn: &Connection, item_id: &str, minutes: &[i64]) -> DbResult<Vec<i64>> {
    require(conn, item_id)?;
    let mut wanted: Vec<i64> = minutes.to_vec();
    wanted.sort_unstable();
    wanted.dedup();
    if let Some(bad) = wanted.iter().find(|m| **m <= 0 || **m > MAX_OFFSET_MINUTES) {
        return Err(DbError::Invalid(format!("reminder offset {bad} minutes")));
    }
    let tx = conn.unchecked_transaction()?;
    for old in get_offsets(&tx, item_id)? {
        if !wanted.contains(&old) {
            tx.execute(
                "DELETE FROM reminder_offsets WHERE item_id = ?1 AND minutes = ?2",
                params![item_id, old],
            )?;
        }
    }
    for m in &wanted {
        tx.execute(
            "INSERT OR IGNORE INTO reminder_offsets (item_id, minutes) VALUES (?1, ?2)",
            params![item_id, m],
        )?;
    }
    tx.commit()?;
    get_offsets(conn, item_id)
}

struct OffsetRow {
    item_id: String,
    minutes: i64,
    fired_for: Option<DateTime<Utc>>,
    due_at: DateTime<Utc>,
}

impl OffsetRow {
    fn fire_at(&self) -> DateTime<Utc> {
        self.due_at - Duration::minutes(self.minutes)
    }
    fn fired(&self) -> bool {
        self.fired_for == Some(self.due_at)
    }
}

fn offset_rows(conn: &Connection) -> DbResult<Vec<OffsetRow>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT o.item_id, o.minutes, o.fired_for, i.due_at FROM reminder_offsets o \
         JOIN items i ON i.id = o.item_id \
         WHERE i.status != 'done' AND i.due_at IS NOT NULL AND {}",
        super::items::VISIBLE
    ))?;
    let rows = stmt.query_map([], |r| {
        Ok(OffsetRow {
            item_id: r.get(0)?,
            minutes: r.get(1)?,
            fired_for: r.get(2)?,
            due_at: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Items whose "before" reminder is due now (the due time itself still ahead),
/// with the offset that fired. Several offsets passed at once (the Mac slept)
/// fire once, as the closest one. Marks them fired.
pub fn take_due_offsets(conn: &Connection, now: DateTime<Utc>) -> DbResult<Vec<(Item, i64)>> {
    let rows = offset_rows(conn)?;
    let mut fired: Vec<(String, i64, DateTime<Utc>)> = Vec::new();
    for row in rows
        .iter()
        .filter(|r| !r.fired() && r.fire_at() <= now && r.due_at > now)
    {
        match fired.iter_mut().find(|(id, _, _)| *id == row.item_id) {
            Some(f) => f.1 = f.1.min(row.minutes),
            None => fired.push((row.item_id.clone(), row.minutes, row.due_at)),
        }
    }
    let mut out = Vec::with_capacity(fired.len());
    for (item_id, minutes, due_at) in fired {
        // Every offset whose time has passed counts as fired for this due time.
        for row in rows
            .iter()
            .filter(|r| r.item_id == item_id && r.fire_at() <= now)
        {
            conn.execute(
                "UPDATE reminder_offsets SET fired_for = ?3 WHERE item_id = ?1 AND minutes = ?2",
                params![item_id, row.minutes, due_at],
            )?;
        }
        if let Some(item) = get_item(conn, &item_id)? {
            out.push((item, minutes));
        }
    }
    Ok(out)
}

/// When the next "before" reminder fires, if any.
pub fn next_offset_at(conn: &Connection, now: DateTime<Utc>) -> DbResult<Option<DateTime<Utc>>> {
    Ok(offset_rows(conn)?
        .iter()
        .filter(|r| !r.fired() && r.due_at > now)
        .map(OffsetRow::fire_at)
        .filter(|t| *t > now)
        .min())
}

// ── Habits ──────────────────────────────────────────────────────────────

pub fn is_habit(conn: &Connection, item_id: &str) -> DbResult<bool> {
    Ok(conn
        .query_row("SELECT 1 FROM habits WHERE item_id = ?1", [item_id], |_| {
            Ok(())
        })
        .optional()?
        .is_some())
}

/// Marks (or unmarks) a recurring item as a habit.
pub fn set_habit(conn: &Connection, item_id: &str, on: bool, now: DateTime<Utc>) -> DbResult<()> {
    let item = require(conn, item_id)?;
    if on {
        if item.recurrence.is_none() {
            return Err(DbError::Invalid(
                "only repeating cards can be habits".into(),
            ));
        }
        conn.execute(
            "INSERT OR IGNORE INTO habits (item_id, created_at) VALUES (?1, ?2)",
            params![item_id, now],
        )?;
    } else {
        conn.execute("DELETE FROM habits WHERE item_id = ?1", [item_id])?;
    }
    Ok(())
}

#[derive(Serialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct HabitStats {
    /// Occurrences done on time in a row, most recent first; 0 while overdue.
    pub streak: u32,
    /// The last 7 local days, oldest first: was an occurrence completed that day?
    pub last7: Vec<bool>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Habit {
    pub item: Item,
    #[serde(flatten)]
    pub stats: HabitStats,
}

fn local_date(t: DateTime<Utc>, tz: Tz) -> NaiveDate {
    t.with_timezone(&tz).date_naive()
}

/// Streak and 7-day strip. An occurrence counts towards the streak when it was
/// completed on (or before) its scheduled local day; a late one ends it. A habit
/// still due before today is overdue, so its streak is 0.
pub fn habit_stats(
    conn: &Connection,
    item: &Item,
    now: DateTime<Utc>,
    tz: Tz,
) -> DbResult<HabitStats> {
    let mut stmt = conn.prepare(
        "SELECT scheduled_at, completed_at FROM item_occurrences \
         WHERE item_id = ?1 AND completed_at IS NOT NULL AND skipped = 0 \
         ORDER BY scheduled_at DESC",
    )?;
    let occurrences: Vec<(DateTime<Utc>, DateTime<Utc>)> = stmt
        .query_map([&item.id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;

    let (today_start, _) = local_day_bounds(now, tz);
    let overdue = item.status != Status::Done
        && item
            .due_at
            .or(item.remind_at)
            .is_some_and(|a| a < today_start);
    let streak = if overdue {
        0
    } else {
        occurrences
            .iter()
            .take_while(|(scheduled, done)| local_date(*done, tz) <= local_date(*scheduled, tz))
            .count() as u32
    };

    let done_days: HashSet<NaiveDate> = occurrences
        .iter()
        .map(|(_, done)| local_date(*done, tz))
        .collect();
    let today = local_date(now, tz);
    let last7 = (0..7)
        .rev()
        .map(|back| done_days.contains(&(today - Duration::days(back))))
        .collect();
    Ok(HabitStats { streak, last7 })
}

/// Open habits with their stats, ordered by next date.
pub fn list_habits(conn: &Connection, now: DateTime<Utc>, tz: Tz) -> DbResult<Vec<Habit>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT h.item_id FROM habits h JOIN items i ON i.id = h.item_id \
         WHERE i.status != 'done' AND i.recurrence IS NOT NULL AND {} \
         ORDER BY COALESCE(i.due_at, i.remind_at), i.created_at",
        super::items::VISIBLE
    ))?;
    let ids: Vec<String> = stmt
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(item) = get_item(conn, &id)? {
            let stats = habit_stats(conn, &item, now, tz)?;
            out.push(Habit { item, stats });
        }
    }
    Ok(out)
}

// ── Focus sessions ──────────────────────────────────────────────────────

pub fn log_focus(
    conn: &Connection,
    item_id: &str,
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
    completed: bool,
) -> DbResult<()> {
    let minutes = (ended_at - started_at).num_minutes().max(0);
    conn.execute(
        "INSERT INTO focus_sessions (id, item_id, started_at, ended_at, minutes, completed) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![new_id(), item_id, started_at, ended_at, minutes, completed],
    )?;
    Ok(())
}

#[derive(Serialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct FocusStats {
    /// Sessions that ran to the end.
    pub sessions: u32,
    /// Minutes focused in total (stopped sessions included).
    pub minutes: u32,
}

pub fn focus_stats(conn: &Connection, item_id: &str) -> DbResult<FocusStats> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(completed), 0), COALESCE(SUM(minutes), 0) \
         FROM focus_sessions WHERE item_id = ?1",
        [item_id],
        |r| {
            Ok(FocusStats {
                sessions: r.get(0)?,
                minutes: r.get(1)?,
            })
        },
    )?)
}

// ── Detail panel summary ────────────────────────────────────────────────

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ItemTime {
    pub offsets: Vec<i64>,
    pub habit: bool,
    pub habit_stats: Option<HabitStats>,
    pub focus: FocusStats,
}

pub fn item_time(
    conn: &Connection,
    item_id: &str,
    now: DateTime<Utc>,
    tz: Tz,
) -> DbResult<ItemTime> {
    let item = require(conn, item_id)?;
    let habit = is_habit(conn, item_id)?;
    Ok(ItemTime {
        offsets: get_offsets(conn, item_id)?,
        habit,
        habit_stats: if habit {
            Some(habit_stats(conn, &item, now, tz)?)
        } else {
            None
        },
        focus: focus_stats(conn, item_id)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{
        self,
        models::{ItemType, NewItem},
    };
    use chrono::TimeZone;

    const UTC: Tz = chrono_tz::UTC;

    fn at(day: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, day, h, 0, 0).unwrap()
    }

    fn item(conn: &Connection, due: DateTime<Utc>, rule: Option<&str>) -> Item {
        db::items::create_item(
            conn,
            NewItem {
                kind: ItemType::Task,
                title: "Walk".into(),
                due_at: Some(due),
                recurrence: rule.map(String::from),
                ..Default::default()
            },
            at(1, 8),
        )
        .unwrap()
    }

    #[test]
    fn offsets_fire_once_per_due_time() {
        let conn = db::open_in_memory().unwrap();
        let it = item(&conn, at(10, 16), None);
        set_offsets(&conn, &it.id, &[60, 24 * 60]).unwrap();
        assert_eq!(next_offset_at(&conn, at(9, 0)).unwrap(), Some(at(9, 16)));
        assert!(take_due_offsets(&conn, at(9, 15)).unwrap().is_empty());
        let fired = take_due_offsets(&conn, at(9, 16)).unwrap();
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].1, 24 * 60);
        assert!(take_due_offsets(&conn, at(9, 17)).unwrap().is_empty());
        assert_eq!(take_due_offsets(&conn, at(10, 15)).unwrap()[0].1, 60);
        // Past the due time nothing fires.
        assert!(take_due_offsets(&conn, at(10, 17)).unwrap().is_empty());
        assert!(set_offsets(&conn, &it.id, &[0]).is_err());
    }

    #[test]
    fn habit_streak_counts_on_time_completions() {
        let conn = db::open_in_memory().unwrap();
        let it = item(&conn, at(2, 9), Some("FREQ=DAILY"));
        set_habit(&conn, &it.id, true, at(1, 8)).unwrap();
        // Day 2 and 3 on time, then day 4 completed late on day 5.
        db::items::complete_item(&conn, &it.id, at(2, 10), UTC).unwrap();
        db::items::complete_item(&conn, &it.id, at(3, 10), UTC).unwrap();
        let habits = list_habits(&conn, at(3, 12), UTC).unwrap();
        assert_eq!(habits[0].stats.streak, 2);
        assert_eq!(
            habits[0].stats.last7,
            vec![false, false, false, false, false, true, true]
        );
        db::items::complete_item(&conn, &it.id, at(5, 10), UTC).unwrap();
        let item = get_item(&conn, &it.id).unwrap().unwrap();
        assert_eq!(habit_stats(&conn, &item, at(5, 12), UTC).unwrap().streak, 0);
    }

    #[test]
    fn only_repeating_items_become_habits() {
        let conn = db::open_in_memory().unwrap();
        let it = item(&conn, at(2, 9), None);
        assert!(set_habit(&conn, &it.id, true, at(1, 8)).is_err());
    }
}
