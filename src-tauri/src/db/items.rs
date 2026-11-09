use std::collections::HashMap;

use chrono::{DateTime, Duration, TimeZone, Utc};
use chrono_tz::Tz;
use rusqlite::{Connection, Row, ToSql, params};

use super::error::{DbError, DbResult};
use super::models::{
    Counts, Item, ItemFilter, ItemPatch, ItemType, NewItem, Priority, Status, new_id,
};
use super::placements::place_item;
use super::recurrence::{next_after, validate_rrule};

const ITEM_COLUMNS: &str = "id, kind, title, body, status, priority, due_at, remind_at, snoozed_until, \
                            recurrence, project_id, created_at, updated_at, completed_at";

fn row_to_item(row: &Row<'_>) -> rusqlite::Result<Item> {
    Ok(Item {
        id: row.get("id")?,
        kind: row.get("kind")?,
        title: row.get("title")?,
        body: row.get("body")?,
        status: row.get("status")?,
        priority: row.get("priority")?,
        due_at: row.get("due_at")?,
        remind_at: row.get("remind_at")?,
        snoozed_until: row.get("snoozed_until")?,
        recurrence: row.get("recurrence")?,
        project_id: row.get("project_id")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        completed_at: row.get("completed_at")?,
    })
}

pub fn create_item(conn: &Connection, input: NewItem, now: DateTime<Utc>) -> DbResult<Item> {
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err(DbError::Invalid("title must not be blank".into()));
    }
    if let Some(rule) = &input.recurrence {
        validate_rrule(rule)?;
    }
    let item = Item {
        id: new_id(),
        kind: input.kind,
        title,
        body: input.body.filter(|b| !b.trim().is_empty()),
        status: Status::Inbox,
        priority: input.priority.unwrap_or(Priority::Normal),
        due_at: input.due_at,
        remind_at: input.remind_at,
        snoozed_until: None,
        recurrence: input.recurrence,
        project_id: input.project_id,
        created_at: now,
        updated_at: now,
        completed_at: None,
    };
    conn.execute(
        &format!(
            "INSERT INTO items ({ITEM_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"
        ),
        params![
            item.id,
            item.kind,
            item.title,
            item.body,
            item.status,
            item.priority,
            item.due_at,
            item.remind_at,
            item.snoozed_until,
            item.recurrence,
            item.project_id,
            item.created_at,
            item.updated_at,
            item.completed_at,
        ],
    )?;

    // A project item lands on its project's board unless a board was given.
    let board_id = match (&input.board_id, &item.project_id) {
        (Some(b), _) => Some(b.clone()),
        (None, Some(p)) => super::projects::get_project(conn, p)?.map(|p| p.board_id),
        (None, None) => None,
    };
    if let Some(board_id) = board_id {
        place_item(conn, &item.id, &board_id, 0.0, 0.0)?;
    }
    Ok(item)
}

pub fn get_item(conn: &Connection, id: &str) -> DbResult<Option<Item>> {
    let mut stmt = conn.prepare(&format!("SELECT {ITEM_COLUMNS} FROM items WHERE id = ?1"))?;
    let mut rows = stmt.query_map([id], row_to_item)?;
    rows.next().transpose().map_err(Into::into)
}

fn require_item(conn: &Connection, id: &str) -> DbResult<Item> {
    get_item(conn, id)?.ok_or_else(|| DbError::NotFound(format!("item {id}")))
}

/// Local-day bounds for `now` in `tz`, as UTC instants.
fn local_day_bounds(now: DateTime<Utc>, tz: Tz) -> (DateTime<Utc>, DateTime<Utc>) {
    let local_date = now.with_timezone(&tz).date_naive();
    let start_local = local_date.and_hms_opt(0, 0, 0).expect("midnight exists");
    let start = tz
        .from_local_datetime(&start_local)
        .earliest()
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or(now);
    let end_local = (local_date + Duration::days(1))
        .and_hms_opt(0, 0, 0)
        .expect("midnight exists");
    let end = tz
        .from_local_datetime(&end_local)
        .earliest()
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or(start + Duration::days(1));
    (start, end)
}

/// Builds the WHERE clause and parameters for a filter.
fn filter_sql(filter: &ItemFilter) -> (String, Vec<Box<dyn ToSql>>) {
    let open = "i.status != 'done'";
    match filter {
        ItemFilter::All => ("1 = 1".into(), vec![]),
        ItemFilter::Inbox => (
            format!(
                "{open} AND i.project_id IS NULL AND NOT EXISTS (SELECT 1 FROM placements p WHERE p.item_id = i.id)"
            ),
            vec![],
        ),
        ItemFilter::Today { now, tz } => {
            let (start, end) = local_day_bounds(*now, *tz);
            (
                format!(
                    "{open} AND ((i.due_at >= ?1 AND i.due_at < ?2) OR (i.remind_at >= ?1 AND i.remind_at < ?2))"
                ),
                vec![Box::new(start), Box::new(end)],
            )
        }
        ItemFilter::Upcoming { now, days } => {
            let end = *now + Duration::days(i64::from(*days));
            (
                format!(
                    "{open} AND ((i.due_at >= ?1 AND i.due_at < ?2) OR (i.remind_at >= ?1 AND i.remind_at < ?2))"
                ),
                vec![Box::new(*now), Box::new(end)],
            )
        }
        ItemFilter::Ideas => (format!("{open} AND i.kind = 'idea'"), vec![]),
        ItemFilter::Notes => (format!("{open} AND i.kind = 'note'"), vec![]),
        ItemFilter::Project(id) => (
            format!("{open} AND i.project_id = ?1"),
            vec![Box::new(id.clone())],
        ),
        ItemFilter::Board(id) => (
            format!(
                "{open} AND EXISTS (SELECT 1 FROM placements p WHERE p.item_id = i.id AND p.board_id = ?1)"
            ),
            vec![Box::new(id.clone())],
        ),
    }
}

pub fn list_items(conn: &Connection, filter: &ItemFilter) -> DbResult<Vec<Item>> {
    let (where_sql, params) = filter_sql(filter);
    let sql = format!(
        "SELECT {} FROM items i WHERE {where_sql} \
         ORDER BY (i.due_at IS NULL), i.due_at ASC, i.created_at DESC",
        ITEM_COLUMNS
            .split(", ")
            .map(|c| format!("i.{c}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let refs: Vec<&dyn ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(refs.as_slice(), row_to_item)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn count_items(conn: &Connection, filter: &ItemFilter) -> DbResult<u32> {
    let (where_sql, params) = filter_sql(filter);
    let sql = format!("SELECT COUNT(*) FROM items i WHERE {where_sql}");
    let refs: Vec<&dyn ToSql> = params.iter().map(|p| p.as_ref()).collect();
    Ok(conn.query_row(&sql, refs.as_slice(), |r| r.get(0))?)
}

pub fn counts(conn: &Connection, now: DateTime<Utc>, tz: Tz) -> DbResult<Counts> {
    let mut by_project = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT project_id, COUNT(*) FROM items WHERE status != 'done' AND project_id IS NOT NULL GROUP BY project_id",
    )?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?)))? {
        let (id, n) = row?;
        by_project.insert(id, n);
    }
    Ok(Counts {
        inbox: count_items(conn, &ItemFilter::Inbox)?,
        today: count_items(conn, &ItemFilter::Today { now, tz })?,
        upcoming: count_items(conn, &ItemFilter::Upcoming { now, days: 7 })?,
        ideas: count_items(conn, &ItemFilter::Ideas)?,
        notes: count_items(conn, &ItemFilter::Notes)?,
        by_project,
    })
}

pub fn update_item(
    conn: &Connection,
    id: &str,
    patch: ItemPatch,
    now: DateTime<Utc>,
) -> DbResult<Item> {
    let mut item = require_item(conn, id)?;
    if let Some(kind) = patch.kind {
        item.kind = kind;
    }
    if let Some(title) = patch.title {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err(DbError::Invalid("title must not be blank".into()));
        }
        item.title = title;
    }
    if let Some(body) = patch.body {
        item.body = body.filter(|b| !b.trim().is_empty());
    }
    if let Some(status) = patch.status {
        item.status = status;
        item.completed_at = if status == Status::Done {
            Some(now)
        } else {
            None
        };
    }
    if let Some(priority) = patch.priority {
        item.priority = priority;
    }
    if let Some(due_at) = patch.due_at {
        item.due_at = due_at;
    }
    if let Some(remind_at) = patch.remind_at {
        item.remind_at = remind_at;
    }
    if let Some(snoozed_until) = patch.snoozed_until {
        item.snoozed_until = snoozed_until;
    }
    if let Some(recurrence) = patch.recurrence {
        if let Some(rule) = &recurrence {
            validate_rrule(rule)?;
        }
        item.recurrence = recurrence;
    }
    if let Some(project_id) = patch.project_id {
        item.project_id = project_id;
    }
    item.updated_at = now;
    write_item(conn, &item)?;
    Ok(item)
}

fn write_item(conn: &Connection, item: &Item) -> DbResult<()> {
    let changed = conn.execute(
        "UPDATE items SET kind = ?2, title = ?3, body = ?4, status = ?5, priority = ?6, due_at = ?7, \
         remind_at = ?8, snoozed_until = ?9, recurrence = ?10, project_id = ?11, updated_at = ?12, \
         completed_at = ?13 WHERE id = ?1",
        params![
            item.id,
            item.kind,
            item.title,
            item.body,
            item.status,
            item.priority,
            item.due_at,
            item.remind_at,
            item.snoozed_until,
            item.recurrence,
            item.project_id,
            item.updated_at,
            item.completed_at,
        ],
    )?;
    if changed == 0 {
        return Err(DbError::NotFound(format!("item {}", item.id)));
    }
    Ok(())
}

pub fn delete_item(conn: &Connection, id: &str) -> DbResult<()> {
    let changed = conn.execute("DELETE FROM items WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(DbError::NotFound(format!("item {id}")));
    }
    Ok(())
}

/// Completes an item. A recurring item logs the occurrence and advances to its
/// next date instead of becoming `done`; an exhausted rule becomes `done`.
pub fn complete_item(conn: &Connection, id: &str, now: DateTime<Utc>) -> DbResult<Item> {
    let mut item = require_item(conn, id)?;
    let anchor = item.due_at.or(item.remind_at);

    if let (Some(rule), Some(anchor)) = (item.recurrence.clone(), anchor) {
        if let Some(next) = next_after(&rule, anchor, now.max(anchor))? {
            conn.execute(
                "INSERT INTO item_occurrences (id, item_id, scheduled_at, completed_at, skipped) \
                 VALUES (?1, ?2, ?3, ?4, 0)",
                params![new_id(), item.id, anchor, now],
            )?;
            let shift = next - anchor;
            item.due_at = item.due_at.map(|d| d + shift);
            item.remind_at = item.remind_at.map(|d| d + shift);
            item.snoozed_until = None;
            item.updated_at = now;
            write_item(conn, &item)?;
            return Ok(item);
        }
    }

    item.status = Status::Done;
    item.completed_at = Some(now);
    item.snoozed_until = None;
    item.updated_at = now;
    write_item(conn, &item)?;
    Ok(item)
}

/// Full-text search over title and body (FTS5 prefix query).
pub fn search_items(conn: &Connection, query: &str, limit: u32) -> DbResult<Vec<Item>> {
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|t| format!("\"{}\"*", t.replace('"', "\"\"")))
        .collect();
    if terms.is_empty() {
        return Ok(vec![]);
    }
    let match_expr = terms.join(" ");
    let sql = format!(
        "SELECT {} FROM items i JOIN items_fts f ON f.rowid = i.rowid \
         WHERE items_fts MATCH ?1 ORDER BY bm25(items_fts) LIMIT ?2",
        ITEM_COLUMNS
            .split(", ")
            .map(|c| format!("i.{c}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![match_expr, limit], row_to_item)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[allow(dead_code)]
pub fn kind_of(item: &Item) -> ItemType {
    item.kind
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn utc(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
    }

    fn task(title: &str) -> NewItem {
        NewItem {
            kind: ItemType::Task,
            title: title.into(),
            ..Default::default()
        }
    }

    #[test]
    fn create_then_get_roundtrip() {
        let conn = db::open_in_memory().unwrap();
        let now = utc(2026, 11, 9, 10, 0);
        let created = create_item(
            &conn,
            NewItem {
                body: Some("Bring the deck".into()),
                due_at: Some(utc(2026, 11, 12, 9, 0)),
                priority: Some(Priority::High),
                ..task("Send revised deck to Marta")
            },
            now,
        )
        .unwrap();
        let fetched = get_item(&conn, &created.id).unwrap().unwrap();
        assert_eq!(fetched, created);
        assert_eq!(fetched.priority, Priority::High);
        assert_eq!(fetched.due_at, Some(utc(2026, 11, 12, 9, 0)));
    }

    #[test]
    fn create_defaults_status_inbox_priority_normal() {
        let conn = db::open_in_memory().unwrap();
        let item = create_item(&conn, task("Hello"), Utc::now()).unwrap();
        assert_eq!(item.status, Status::Inbox);
        assert_eq!(item.priority, Priority::Normal);
        assert!(item.completed_at.is_none());
    }

    #[test]
    fn blank_title_rejected() {
        let conn = db::open_in_memory().unwrap();
        assert!(create_item(&conn, task("  "), Utc::now()).is_err());
    }

    #[test]
    fn list_inbox_excludes_done_and_projects() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let project = db::projects::create_project(&conn, "P", "#79B8F5", now).unwrap();
        let a = create_item(&conn, task("inbox a"), now).unwrap();
        let b = create_item(&conn, task("done b"), now).unwrap();
        complete_item(&conn, &b.id, now).unwrap();
        create_item(
            &conn,
            NewItem {
                project_id: Some(project.id.clone()),
                ..task("in project")
            },
            now,
        )
        .unwrap();
        let inbox = list_items(&conn, &ItemFilter::Inbox).unwrap();
        assert_eq!(
            inbox.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
            vec![a.id.as_str()]
        );
        assert_eq!(
            list_items(&conn, &ItemFilter::Project(project.id.clone()))
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            list_items(&conn, &ItemFilter::Board(project.board_id.clone()))
                .unwrap()
                .len(),
            1
        );
        assert_eq!(list_items(&conn, &ItemFilter::All).unwrap().len(), 3);
    }

    #[test]
    fn list_today_uses_local_day() {
        let conn = db::open_in_memory().unwrap();
        let tz: Tz = "Europe/Rome".parse().unwrap();
        // 2026-11-09 12:00 Rome = 11:00 UTC.
        let now = utc(2026, 11, 9, 11, 0);
        // 23:30 Rome on the 9th = 22:30 UTC → today.
        let late = create_item(
            &conn,
            NewItem {
                due_at: Some(utc(2026, 11, 9, 22, 30)),
                ..task("late")
            },
            now,
        )
        .unwrap();
        // 00:30 Rome on the 10th = 23:30 UTC on the 9th → tomorrow.
        create_item(
            &conn,
            NewItem {
                due_at: Some(utc(2026, 11, 9, 23, 30)),
                ..task("tomorrow")
            },
            now,
        )
        .unwrap();
        // Reminder at 09:00 Rome today = 08:00 UTC → today even though due_at is null.
        let rem = create_item(
            &conn,
            NewItem {
                kind: ItemType::Reminder,
                remind_at: Some(utc(2026, 11, 9, 8, 0)),
                ..task("rem")
            },
            now,
        )
        .unwrap();
        let today = list_items(&conn, &ItemFilter::Today { now, tz }).unwrap();
        let ids: Vec<&str> = today.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&late.id.as_str()));
        assert!(ids.contains(&rem.id.as_str()));
    }

    #[test]
    fn list_upcoming_within_days() {
        let conn = db::open_in_memory().unwrap();
        let now = utc(2026, 11, 9, 11, 0);
        let soon = create_item(
            &conn,
            NewItem {
                due_at: Some(utc(2026, 11, 14, 9, 0)),
                ..task("soon")
            },
            now,
        )
        .unwrap();
        create_item(
            &conn,
            NewItem {
                due_at: Some(utc(2026, 11, 30, 9, 0)),
                ..task("far")
            },
            now,
        )
        .unwrap();
        create_item(
            &conn,
            NewItem {
                due_at: Some(utc(2026, 11, 1, 9, 0)),
                ..task("past")
            },
            now,
        )
        .unwrap();
        let up = list_items(&conn, &ItemFilter::Upcoming { now, days: 7 }).unwrap();
        assert_eq!(
            up.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
            vec![soon.id.as_str()]
        );
    }

    #[test]
    fn update_patch_only_touches_given_fields() {
        let conn = db::open_in_memory().unwrap();
        let now = utc(2026, 11, 9, 11, 0);
        let item = create_item(
            &conn,
            NewItem {
                body: Some("keep me".into()),
                due_at: Some(utc(2026, 11, 12, 9, 0)),
                ..task("Title")
            },
            now,
        )
        .unwrap();
        let later = now + Duration::minutes(5);
        let patch: ItemPatch =
            serde_json::from_str(r#"{"title":"New title","dueAt":null}"#).unwrap();
        let updated = update_item(&conn, &item.id, patch, later).unwrap();
        assert_eq!(updated.title, "New title");
        assert_eq!(updated.body.as_deref(), Some("keep me"));
        assert_eq!(updated.due_at, None);
        assert_eq!(updated.updated_at, later);
        assert_eq!(updated.created_at, now);
    }

    #[test]
    fn complete_sets_done_and_completed_at() {
        let conn = db::open_in_memory().unwrap();
        let now = utc(2026, 11, 9, 11, 0);
        let item = create_item(&conn, task("Do it"), now).unwrap();
        let done = complete_item(&conn, &item.id, now).unwrap();
        assert_eq!(done.status, Status::Done);
        assert_eq!(done.completed_at, Some(now));
        assert!(list_items(&conn, &ItemFilter::Inbox).unwrap().is_empty());
    }

    #[test]
    fn complete_recurring_weekly_advances_due_and_logs_occurrence() {
        let conn = db::open_in_memory().unwrap();
        let due = utc(2026, 11, 9, 9, 0); // Monday
        let item = create_item(
            &conn,
            NewItem {
                due_at: Some(due),
                remind_at: Some(due - Duration::hours(1)),
                recurrence: Some("FREQ=WEEKLY;BYDAY=MO".into()),
                ..task("Weekly review")
            },
            due,
        )
        .unwrap();
        let now = utc(2026, 11, 9, 10, 0);
        let next = complete_item(&conn, &item.id, now).unwrap();
        assert_eq!(next.status, Status::Inbox);
        assert_eq!(next.due_at, Some(utc(2026, 11, 16, 9, 0)));
        assert_eq!(next.remind_at, Some(utc(2026, 11, 16, 8, 0)));
        assert!(next.completed_at.is_none());
        let occ: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM item_occurrences WHERE item_id = ?1",
                [&item.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(occ, 1);
    }

    #[test]
    fn complete_recurring_exhausted_marks_done() {
        let conn = db::open_in_memory().unwrap();
        let due = utc(2026, 11, 9, 9, 0);
        let item = create_item(
            &conn,
            NewItem {
                due_at: Some(due),
                recurrence: Some("FREQ=DAILY;COUNT=1".into()),
                ..task("Once")
            },
            due,
        )
        .unwrap();
        let done = complete_item(&conn, &item.id, utc(2026, 11, 9, 10, 0)).unwrap();
        assert_eq!(done.status, Status::Done);
        assert!(done.completed_at.is_some());
    }

    #[test]
    fn invalid_recurrence_rejected_on_create() {
        let conn = db::open_in_memory().unwrap();
        let err = create_item(
            &conn,
            NewItem {
                recurrence: Some("FREQ=HOURLY".into()),
                ..task("x")
            },
            Utc::now(),
        );
        assert!(matches!(err, Err(DbError::InvalidRecurrence(_))));
    }

    #[test]
    fn delete_removes_placement_cascade() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let board = db::boards::create_board(&conn, "B", now).unwrap();
        let item = create_item(
            &conn,
            NewItem {
                board_id: Some(board.id.clone()),
                ..task("on board")
            },
            now,
        )
        .unwrap();
        assert_eq!(
            db::placements::list_placements(&conn, &board.id)
                .unwrap()
                .len(),
            1
        );
        delete_item(&conn, &item.id).unwrap();
        assert!(
            db::placements::list_placements(&conn, &board.id)
                .unwrap()
                .is_empty()
        );
        assert!(matches!(
            delete_item(&conn, &item.id),
            Err(DbError::NotFound(_))
        ));
    }

    #[test]
    fn fts_finds_by_body_word() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let hit = create_item(
            &conn,
            NewItem {
                kind: ItemType::Note,
                body: Some("She keeps reminders in three apps and trusts none of them.".into()),
                ..task("Interview notes")
            },
            now,
        )
        .unwrap();
        create_item(&conn, task("Unrelated"), now).unwrap();
        let found = search_items(&conn, "remind", 10).unwrap();
        assert_eq!(
            found.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
            vec![hit.id.as_str()]
        );
        assert!(search_items(&conn, "", 10).unwrap().is_empty());
    }

    #[test]
    fn counts_by_project() {
        let conn = db::open_in_memory().unwrap();
        let now = utc(2026, 11, 9, 11, 0);
        let tz: Tz = "Europe/Rome".parse().unwrap();
        let p = db::projects::create_project(&conn, "P", "#79B8F5", now).unwrap();
        create_item(
            &conn,
            NewItem {
                project_id: Some(p.id.clone()),
                ..task("a")
            },
            now,
        )
        .unwrap();
        create_item(
            &conn,
            NewItem {
                project_id: Some(p.id.clone()),
                ..task("b")
            },
            now,
        )
        .unwrap();
        create_item(
            &conn,
            NewItem {
                kind: ItemType::Idea,
                ..task("idea")
            },
            now,
        )
        .unwrap();
        let c = counts(&conn, now, tz).unwrap();
        assert_eq!(c.by_project.get(&p.id), Some(&2));
        assert_eq!(c.ideas, 1);
        assert_eq!(c.inbox, 1);
    }
}
