use std::collections::HashMap;

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use rusqlite::{Connection, OptionalExtension, Row, ToSql, params};

use super::error::{DbError, DbResult};
use super::models::{
    BoardKind, CaptureHint, Counts, Item, ItemFilter, ItemPatch, NewItem, Priority, Progress,
    Status, new_id,
};
use super::placements::{place_item, remove_placement};
use super::recurrence::{next_after, validate_rrule};
use super::tags::{attach_tags, set_item_tags};

/// Columns selected for every item query, aliased so `row_to_item` can read by name.
pub(crate) const SELECT_ITEM: &str = "SELECT i.id, i.kind, i.title, i.body, i.status, i.priority, i.due_at, \
    i.ends_at, i.remind_at, i.snoozed_until, i.recurrence, i.project_id, i.created_at, \
    i.updated_at, i.completed_at, \
    i.hint, i.pinned, i.archived_at, i.deleted_at, \
    (SELECT COUNT(*) FROM subtasks st WHERE st.item_id = i.id) AS subtasks_total, \
    (SELECT COUNT(*) FROM subtasks st WHERE st.item_id = i.id AND st.done = 1) AS subtasks_done, \
    (SELECT COUNT(*) FROM attachments att WHERE att.item_id = i.id) AS attachment_count, \
    (SELECT COUNT(*) FROM connections c WHERE c.from_item = i.id OR c.to_item = i.id) AS link_count, \
    EXISTS (SELECT 1 FROM captures k WHERE k.item_id = i.id AND k.status IN ('pending', 'processing')) AS capturing \
    FROM items i";

/// Canvas auto-slot grid for new cards.
const SLOT_COLUMNS: u32 = 4;
const SLOT_W: f64 = 300.0;
const SLOT_H: f64 = 200.0;
const SLOT_ORIGIN: f64 = 80.0;

fn row_to_item(row: &Row<'_>) -> rusqlite::Result<Item> {
    Ok(Item {
        id: row.get("id")?,
        kind: row.get("kind")?,
        title: row.get("title")?,
        body: row.get("body")?,
        status: row.get("status")?,
        priority: row.get("priority")?,
        due_at: row.get("due_at")?,
        ends_at: row.get("ends_at")?,
        remind_at: row.get("remind_at")?,
        snoozed_until: row.get("snoozed_until")?,
        recurrence: row.get("recurrence")?,
        project_id: row.get("project_id")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        completed_at: row.get("completed_at")?,
        tags: Vec::new(),
        link_count: row.get("link_count")?,
        hint: row
            .get::<_, Option<String>>("hint")?
            .and_then(|h| serde_json::from_str(&h).ok()),
        capturing: row.get("capturing")?,
        pinned: row.get("pinned")?,
        archived_at: row.get("archived_at")?,
        deleted_at: row.get("deleted_at")?,
        subtasks_done: row.get("subtasks_done")?,
        subtasks_total: row.get("subtasks_total")?,
        attachment_count: row.get("attachment_count")?,
    })
}

/// Items every view shows: not archived, not in the Trash, not in an archived project.
pub(crate) const VISIBLE: &str = "i.archived_at IS NULL AND i.deleted_at IS NULL \
    AND NOT EXISTS (SELECT 1 FROM projects ap WHERE ap.id = i.project_id AND ap.archived_at IS NOT NULL)";

fn hint_json(hint: &Option<CaptureHint>) -> Option<String> {
    hint.as_ref()
        .filter(|h| !h.is_empty())
        .and_then(|h| serde_json::to_string(h).ok())
}

pub(crate) fn query_items(
    conn: &Connection,
    sql: &str,
    params: &[&dyn ToSql],
) -> DbResult<Vec<Item>> {
    let mut stmt = conn.prepare_cached(sql)?;
    let rows = stmt.query_map(params, row_to_item)?;
    let mut items: Vec<Item> = rows.collect::<Result<_, _>>()?;
    attach_tags(conn, &mut items)?;
    Ok(items)
}

/// Next free slot on `board_id`: the first cell of a 4-column grid that no
/// existing card overlaps (cards placed by hand can sit anywhere).
pub(crate) fn next_slot(conn: &Connection, board_id: &str) -> DbResult<(f64, f64)> {
    let mut stmt = conn.prepare_cached("SELECT x, y FROM placements WHERE board_id = ?1")?;
    let taken: Vec<(f64, f64)> = stmt
        .query_map([board_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    Ok(free_slot(&taken))
}

/// First grid cell not overlapping any of `taken` (approximate card box 280 x 180).
pub fn free_slot(taken: &[(f64, f64)]) -> (f64, f64) {
    (0u32..)
        .map(|n| {
            let col = f64::from(n % SLOT_COLUMNS);
            let row = f64::from(n / SLOT_COLUMNS);
            (SLOT_ORIGIN + col * SLOT_W, SLOT_ORIGIN + row * SLOT_H)
        })
        .find(|(x, y)| {
            taken
                .iter()
                .all(|(tx, ty)| (tx - x).abs() >= 280.0 || (ty - y).abs() >= 180.0)
        })
        .expect("an unbounded grid always has a free cell")
}

fn project_board(conn: &Connection, project_id: &str) -> DbResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT board_id FROM projects WHERE id = ?1",
            [project_id],
            |r| r.get(0),
        )
        .optional()?)
}

fn current_placement_board(conn: &Connection, item_id: &str) -> DbResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT board_id FROM placements WHERE item_id = ?1",
            [item_id],
            |r| r.get(0),
        )
        .optional()?)
}

fn board_kind(conn: &Connection, board_id: &str) -> DbResult<Option<BoardKind>> {
    Ok(conn
        .query_row("SELECT kind FROM boards WHERE id = ?1", [board_id], |r| {
            r.get(0)
        })
        .optional()?)
}

pub fn create_item(conn: &Connection, input: NewItem, now: DateTime<Utc>) -> DbResult<Item> {
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err(DbError::Invalid("title must not be blank".into()));
    }
    if let Some(rule) = &input.recurrence {
        validate_rrule(rule)?;
    }
    let status = input.status.unwrap_or(Status::Inbox);
    let tx = conn.unchecked_transaction()?;
    let id = new_id();
    let recurrence_start = input
        .recurrence
        .as_ref()
        .and(input.due_at.or(input.remind_at));
    tx.execute(
        "INSERT INTO items (id, kind, title, body, status, priority, due_at, ends_at, remind_at, \
         snoozed_until, recurrence, recurrence_start, project_id, created_at, updated_at, completed_at, hint) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, ?10, ?11, ?12, ?13, ?13, ?14, ?15)",
        params![
            id,
            input.kind,
            title,
            input.body.filter(|b| !b.trim().is_empty()),
            status,
            input.priority.unwrap_or(Priority::Normal),
            input.due_at,
            input.ends_at,
            input.remind_at,
            input.recurrence,
            recurrence_start,
            input.project_id,
            now,
            (status == Status::Done).then_some(now),
            hint_json(&input.hint),
        ],
    )?;

    if let Some(tags) = &input.tags {
        set_item_tags(&tx, &id, tags)?;
    }

    // A project item lands on its project's board unless a board was given.
    let board_id = match (&input.board_id, &input.project_id) {
        (Some(b), _) => Some(b.clone()),
        (None, Some(p)) => project_board(&tx, p)?,
        (None, None) => None,
    };
    if let Some(board_id) = board_id {
        let (x, y) = match (input.x, input.y) {
            (Some(x), Some(y)) => (x, y),
            _ => next_slot(&tx, &board_id)?,
        };
        place_item(&tx, &id, &board_id, x, y)?;
    }
    tx.commit()?;
    require_item(conn, &id)
}

pub fn get_item(conn: &Connection, id: &str) -> DbResult<Option<Item>> {
    let sql = format!("{SELECT_ITEM} WHERE i.id = ?1");
    Ok(query_items(conn, &sql, &[&id])?.into_iter().next())
}

fn require_item(conn: &Connection, id: &str) -> DbResult<Item> {
    get_item(conn, id)?.ok_or_else(|| DbError::NotFound(format!("item {id}")))
}

/// Maps a local wall-clock midnight to UTC, stepping forward through a DST gap
/// (some zones skip 00:00 when DST starts).
fn local_midnight(date: NaiveDate, tz: Tz) -> Option<DateTime<Utc>> {
    (0..=3).find_map(|h| {
        let local = date.and_hms_opt(h, 0, 0)?;
        tz.from_local_datetime(&local)
            .earliest()
            .map(|d| d.with_timezone(&Utc))
    })
}

/// Local-day bounds for `now` in `tz`, as UTC instants.
pub fn local_day_bounds(now: DateTime<Utc>, tz: Tz) -> (DateTime<Utc>, DateTime<Utc>) {
    let date = now.with_timezone(&tz).date_naive();
    let start = local_midnight(date, tz).unwrap_or(now - Duration::hours(12));
    let end = local_midnight(date + Duration::days(1), tz).unwrap_or(start + Duration::days(1));
    (start, end)
}

/// WHERE clause and parameters for a filter. Today includes overdue open items;
/// the Upcoming window is supplied by the caller (it starts where Today ends).
fn filter_sql(filter: &ItemFilter, include_done: bool) -> (String, Vec<Box<dyn ToSql>>) {
    let open = if include_done {
        "1 = 1"
    } else {
        "i.status != 'done'"
    };
    let not_project_card = "i.kind != 'project_ref'";
    match filter {
        ItemFilter::All => (open.to_string(), vec![]),
        ItemFilter::Inbox => (
            format!(
                "{open} AND {not_project_card} AND i.project_id IS NULL \
                 AND NOT EXISTS (SELECT 1 FROM placements p WHERE p.item_id = i.id)"
            ),
            vec![],
        ),
        ItemFilter::Today { now, tz } => {
            let (start, end) = local_day_bounds(*now, *tz);
            // Open items due/reminded before tonight (overdue included); done items
            // only when they were completed today and the caller asked for them.
            let done_today = if include_done {
                "(i.status = 'done' AND i.completed_at >= ?1 AND i.completed_at < ?2 \
                  AND (i.due_at < ?2 OR i.remind_at < ?2))"
            } else {
                "1 = 0"
            };
            (
                format!(
                    "{not_project_card} AND ( \
                       (i.status != 'done' AND (i.due_at < ?2 OR i.remind_at < ?2)) \
                       OR {done_today} )"
                ),
                vec![Box::new(start), Box::new(end)],
            )
        }
        ItemFilter::Upcoming { now, days } => {
            let end = *now + Duration::days(i64::from(*days));
            (
                format!(
                    "{open} AND {not_project_card} AND ((i.due_at >= ?1 AND i.due_at < ?2) \
                     OR (i.remind_at >= ?1 AND i.remind_at < ?2))"
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

pub fn list_items(
    conn: &Connection,
    filter: &ItemFilter,
    include_done: bool,
) -> DbResult<Vec<Item>> {
    let (where_sql, params) = filter_sql(filter, include_done);
    let sql = format!(
        "{SELECT_ITEM} WHERE {VISIBLE} AND ({where_sql}) \
         ORDER BY i.pinned DESC, (i.status = 'done'), (COALESCE(i.due_at, i.remind_at) IS NULL), \
         COALESCE(i.due_at, i.remind_at) ASC, i.created_at DESC"
    );
    let refs: Vec<&dyn ToSql> = params.iter().map(|p| p.as_ref()).collect();
    query_items(conn, &sql, &refs)
}

fn count_items(conn: &Connection, filter: &ItemFilter) -> DbResult<u32> {
    let (where_sql, params) = filter_sql(filter, false);
    let sql = format!("SELECT COUNT(*) FROM items i WHERE {VISIBLE} AND ({where_sql})");
    let refs: Vec<&dyn ToSql> = params.iter().map(|p| p.as_ref()).collect();
    Ok(conn.query_row(&sql, refs.as_slice(), |r| r.get(0))?)
}

/// The Upcoming filter as the app uses it: the `days` after today, local time.
pub fn upcoming_filter(now: DateTime<Utc>, tz: Tz, days: u32) -> ItemFilter {
    let (_, end_of_today) = local_day_bounds(now, tz);
    ItemFilter::Upcoming {
        now: end_of_today,
        days,
    }
}

pub fn counts(conn: &Connection, now: DateTime<Utc>, tz: Tz) -> DbResult<Counts> {
    let mut by_project = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT project_id, COUNT(*) FROM items \
         WHERE status != 'done' AND project_id IS NOT NULL AND kind != 'project_ref' \
         AND archived_at IS NULL AND deleted_at IS NULL \
         GROUP BY project_id",
    )?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?)))? {
        let (id, n) = row?;
        by_project.insert(id, n);
    }
    Ok(Counts {
        inbox: count_items(conn, &ItemFilter::Inbox)?,
        today: count_items(conn, &ItemFilter::Today { now, tz })?,
        upcoming: count_items(conn, &upcoming_filter(now, tz, 7))?,
        ideas: count_items(conn, &ItemFilter::Ideas)?,
        notes: count_items(conn, &ItemFilter::Notes)?,
        by_project,
    })
}

/// Done / total items per project (project cards themselves excluded).
pub fn project_progress(conn: &Connection) -> DbResult<HashMap<String, Progress>> {
    let mut stmt = conn.prepare(
        "SELECT project_id, SUM(status = 'done'), COUNT(*) FROM items \
         WHERE project_id IS NOT NULL AND kind != 'project_ref' \
         AND archived_at IS NULL AND deleted_at IS NULL GROUP BY project_id",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            Progress {
                done: r.get(1)?,
                total: r.get(2)?,
            },
        ))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn update_item(
    conn: &Connection,
    id: &str,
    patch: ItemPatch,
    now: DateTime<Utc>,
    tz: Tz,
) -> DbResult<Item> {
    // "Done" goes through completion so recurring items advance instead.
    if patch.status == Some(Status::Done) {
        let mut rest = patch.clone();
        rest.status = None;
        if rest != ItemPatch::default() {
            update_item(conn, id, rest, now, tz)?;
        }
        return complete_item(conn, id, now, tz);
    }

    let tx = conn.unchecked_transaction()?;
    let mut item = require_item(&tx, id)?;
    let old_project = item.project_id.clone();
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
        item.completed_at = None;
    }
    if let Some(priority) = patch.priority {
        item.priority = priority;
    }
    if let Some(due_at) = patch.due_at {
        item.due_at = due_at;
    }
    if let Some(ends_at) = patch.ends_at {
        item.ends_at = ends_at;
    }
    if let Some(remind_at) = patch.remind_at {
        item.remind_at = remind_at;
    }
    if let Some(snoozed_until) = patch.snoozed_until {
        item.snoozed_until = snoozed_until;
    }
    let mut restart_series = patch.due_at.is_some() || patch.remind_at.is_some();
    if let Some(recurrence) = patch.recurrence {
        if let Some(rule) = &recurrence {
            validate_rrule(rule)?;
        }
        item.recurrence = recurrence;
        restart_series = true;
    }
    let project_patched = patch.project_id.is_some();
    if let Some(project_id) = patch.project_id {
        item.project_id = project_id;
    }
    if let Some(hint) = patch.hint {
        item.hint = hint;
    }
    // Editing a date resolves a "check date" hint; choosing a project resolves "new project?".
    if let Some(h) = item.hint.as_mut() {
        if patch.due_at.is_some() || patch.remind_at.is_some() {
            h.date_phrase = None;
        }
        if project_patched {
            h.suggested_project = None;
        }
        if h.is_empty() {
            item.hint = None;
        }
    }
    item.updated_at = now;
    write_item(&tx, &item)?;

    if restart_series {
        let start = item.recurrence.as_ref().and(item.due_at.or(item.remind_at));
        tx.execute(
            "UPDATE items SET recurrence_start = ?2 WHERE id = ?1",
            params![item.id, start],
        )?;
    }

    if let Some(tags) = &patch.tags {
        set_item_tags(&tx, &item.id, tags)?;
    }

    // Changing project moves the card to the new project's board, or off the old
    // project's board when cleared. Personal-board placements are left alone.
    if item.project_id != old_project {
        let current = current_placement_board(&tx, &item.id)?;
        let on_project_board = match &current {
            Some(b) => board_kind(&tx, b)? == Some(BoardKind::Project),
            None => true,
        };
        if on_project_board {
            match &item.project_id {
                Some(p) => {
                    if let Some(board) = project_board(&tx, p)? {
                        let (x, y) = next_slot(&tx, &board)?;
                        place_item(&tx, &item.id, &board, x, y)?;
                    }
                }
                None => remove_placement(&tx, &item.id)?,
            }
        }
    }
    tx.commit()?;
    require_item(conn, id)
}

fn write_item(conn: &Connection, item: &Item) -> DbResult<()> {
    let changed = conn.execute(
        "UPDATE items SET kind = ?2, title = ?3, body = ?4, status = ?5, priority = ?6, due_at = ?7, \
         ends_at = ?8, remind_at = ?9, snoozed_until = ?10, recurrence = ?11, project_id = ?12, \
         updated_at = ?13, completed_at = ?14, hint = ?15 WHERE id = ?1",
        params![
            item.id,
            item.kind,
            item.title,
            item.body,
            item.status,
            item.priority,
            item.due_at,
            item.ends_at,
            item.remind_at,
            item.snoozed_until,
            item.recurrence,
            item.project_id,
            item.updated_at,
            item.completed_at,
            hint_json(&item.hint),
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
/// next date (computed from the fixed series start in `tz`) instead of becoming
/// `done`; an exhausted series becomes `done`. Completing a done item is a no-op.
pub fn complete_item(conn: &Connection, id: &str, now: DateTime<Utc>, tz: Tz) -> DbResult<Item> {
    let tx = conn.unchecked_transaction()?;
    let mut item = require_item(&tx, id)?;
    if item.status == Status::Done {
        return Ok(item);
    }
    let anchor = item.due_at.or(item.remind_at);
    let series_start: Option<DateTime<Utc>> = tx.query_row(
        "SELECT recurrence_start FROM items WHERE id = ?1",
        [id],
        |r| r.get(0),
    )?;

    if let (Some(rule), Some(anchor)) = (item.recurrence.clone(), anchor)
        && let Some(next) = next_after(&rule, series_start.unwrap_or(anchor), anchor, tz)?
    {
        tx.execute(
            "INSERT INTO item_occurrences (id, item_id, scheduled_at, completed_at, skipped) \
             VALUES (?1, ?2, ?3, ?4, 0)",
            params![new_id(), item.id, anchor, now],
        )?;
        let shift = next - anchor;
        item.due_at = item.due_at.map(|d| d + shift);
        item.ends_at = item.ends_at.map(|d| d + shift);
        item.remind_at = item.remind_at.map(|d| d + shift);
        item.snoozed_until = None;
        item.updated_at = now;
        write_item(&tx, &item)?;
        tx.commit()?;
        return require_item(conn, id);
    }

    item.status = Status::Done;
    item.completed_at = Some(now);
    item.snoozed_until = None;
    item.updated_at = now;
    write_item(&tx, &item)?;
    tx.commit()?;
    require_item(conn, id)
}

/// Copies an item (title, body, type, dates, project, tags) as a new open card,
/// placed 24 px below-right of the original when it is on a board.
pub fn duplicate_item(conn: &Connection, id: &str, now: DateTime<Utc>) -> DbResult<Item> {
    let original = require_item(conn, id)?;
    let placement: Option<(String, f64, f64)> = conn
        .query_row(
            "SELECT board_id, x, y FROM placements WHERE item_id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    create_item(
        conn,
        NewItem {
            kind: original.kind,
            title: original.title.clone(),
            body: original.body.clone(),
            priority: Some(original.priority),
            status: None,
            due_at: original.due_at,
            ends_at: original.ends_at,
            remind_at: original.remind_at,
            recurrence: original.recurrence.clone(),
            project_id: original.project_id.clone(),
            board_id: placement.as_ref().map(|p| p.0.clone()),
            tags: Some(original.tags.clone()),
            x: placement.as_ref().map(|p| p.1 + 24.0),
            y: placement.as_ref().map(|p| p.2 + 24.0),
            hint: None,
        },
        now,
    )
}

/// Full-text search over title and body (FTS5 prefix query).
/// Titles of the latest cards in a project (`board = false`) or on a board
/// (`board = true`), newest first. Used to tell the AI what a place is about.
pub fn recent_titles(
    conn: &Connection,
    id: &str,
    board: bool,
    limit: u32,
) -> DbResult<Vec<String>> {
    let sql = if board {
        "SELECT i.title FROM items i JOIN placements p ON p.item_id = i.id \
         WHERE p.board_id = ?1 AND i.kind != 'project_ref' AND i.deleted_at IS NULL \
         ORDER BY i.updated_at DESC LIMIT ?2"
    } else {
        "SELECT title FROM items WHERE project_id = ?1 AND kind != 'project_ref' \
         AND deleted_at IS NULL ORDER BY updated_at DESC LIMIT ?2"
    };
    let mut stmt = conn.prepare_cached(sql)?;
    let rows = stmt.query_map(params![id, limit], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

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
        "{SELECT_ITEM} JOIN items_fts f ON f.id = i.id \
         WHERE items_fts MATCH ?1 AND {VISIBLE} \
         ORDER BY (i.status = 'done'), bm25(items_fts) LIMIT ?2"
    );
    query_items(conn, &sql, &[&match_expr, &limit])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{self, models::ItemType};

    const UTC_TZ: Tz = chrono_tz::UTC;

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

    fn ids(items: &[Item]) -> Vec<&str> {
        items.iter().map(|i| i.id.as_str()).collect()
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
    fn failed_placement_leaves_no_ghost_item() {
        let conn = db::open_in_memory().unwrap();
        let err = create_item(
            &conn,
            NewItem {
                board_id: Some("missing-board".into()),
                ..task("ghost")
            },
            Utc::now(),
        );
        assert!(err.is_err());
        assert!(
            list_items(&conn, &ItemFilter::All, true)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn new_board_items_take_free_slots() {
        let conn = db::open_in_memory().unwrap();
        let board = db::boards::create_board(&conn, "B", Utc::now()).unwrap();
        for t in ["a", "b", "c", "d", "e"] {
            create_item(
                &conn,
                NewItem {
                    board_id: Some(board.id.clone()),
                    ..task(t)
                },
                Utc::now(),
            )
            .unwrap();
        }
        let mut pos: Vec<(f64, f64)> = db::placements::list_placements(&conn, &board.id)
            .unwrap()
            .iter()
            .map(|p| (p.x, p.y))
            .collect();
        pos.sort_by(|a, b| a.partial_cmp(b).unwrap());
        pos.dedup();
        assert_eq!(pos.len(), 5);
    }

    #[test]
    fn free_slot_skips_cells_covered_by_hand_placed_cards() {
        assert_eq!(free_slot(&[]), (SLOT_ORIGIN, SLOT_ORIGIN));
        // A card dragged near the first cell pushes the next card to the second.
        let (x, y) = free_slot(&[(100.0, 90.0)]);
        assert_eq!((x, y), (SLOT_ORIGIN + SLOT_W, SLOT_ORIGIN));
    }

    #[test]
    fn list_inbox_excludes_done_and_projects() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let project = db::projects::create_project(&conn, "P", "#79B8F5", now).unwrap();
        let a = create_item(&conn, task("inbox a"), now).unwrap();
        let b = create_item(&conn, task("done b"), now).unwrap();
        complete_item(&conn, &b.id, now, UTC_TZ).unwrap();
        create_item(
            &conn,
            NewItem {
                project_id: Some(project.id.clone()),
                ..task("in project")
            },
            now,
        )
        .unwrap();
        let inbox = list_items(&conn, &ItemFilter::Inbox, false).unwrap();
        assert_eq!(ids(&inbox), vec![a.id.as_str()]);
        assert_eq!(
            list_items(&conn, &ItemFilter::Project(project.id.clone()), false)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            list_items(&conn, &ItemFilter::Board(project.board_id.clone()), false)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(list_items(&conn, &ItemFilter::All, true).unwrap().len(), 3);
        assert_eq!(list_items(&conn, &ItemFilter::All, false).unwrap().len(), 2);
        assert_eq!(
            list_items(&conn, &ItemFilter::Inbox, true).unwrap().len(),
            2
        );
    }

    #[test]
    fn list_today_uses_local_day_and_includes_overdue() {
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
        // Reminder at 09:00 Rome today = 08:00 UTC → today.
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
        // Due yesterday and still open → overdue, shown today.
        let overdue = create_item(
            &conn,
            NewItem {
                due_at: Some(utc(2026, 11, 8, 16, 0)),
                ..task("overdue")
            },
            now,
        )
        .unwrap();
        let today = list_items(&conn, &ItemFilter::Today { now, tz }, false).unwrap();
        let got = ids(&today);
        assert_eq!(got.len(), 3);
        for id in [&late.id, &rem.id, &overdue.id] {
            assert!(got.contains(&id.as_str()));
        }
    }

    #[test]
    fn today_with_done_shows_items_completed_today() {
        let conn = db::open_in_memory().unwrap();
        let now = utc(2026, 11, 9, 11, 0);
        let item = create_item(
            &conn,
            NewItem {
                due_at: Some(utc(2026, 11, 9, 9, 0)),
                ..task("done today")
            },
            now,
        )
        .unwrap();
        complete_item(&conn, &item.id, now, UTC_TZ).unwrap();
        let filter = ItemFilter::Today { now, tz: UTC_TZ };
        assert!(list_items(&conn, &filter, false).unwrap().is_empty());
        assert_eq!(list_items(&conn, &filter, true).unwrap().len(), 1);
    }

    #[test]
    fn upcoming_starts_after_today() {
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
        for (t, d) in [
            ("later today", utc(2026, 11, 9, 18, 0)),
            ("far", utc(2026, 11, 30, 9, 0)),
            ("past", utc(2026, 11, 1, 9, 0)),
        ] {
            create_item(
                &conn,
                NewItem {
                    due_at: Some(d),
                    ..task(t)
                },
                now,
            )
            .unwrap();
        }
        let up = list_items(&conn, &upcoming_filter(now, UTC_TZ, 7), false).unwrap();
        assert_eq!(ids(&up), vec![soon.id.as_str()]);
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
            serde_json::from_str(r#"{"title":"New title","dueAt":null,"tags":["x"]}"#).unwrap();
        let updated = update_item(&conn, &item.id, patch, later, UTC_TZ).unwrap();
        assert_eq!(updated.title, "New title");
        assert_eq!(updated.body.as_deref(), Some("keep me"));
        assert_eq!(updated.due_at, None);
        assert_eq!(updated.tags, vec!["x".to_string()]);
        assert_eq!(updated.updated_at, later);
        assert_eq!(updated.created_at, now);
    }

    #[test]
    fn changing_project_moves_board_placement() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let a = db::projects::create_project(&conn, "A", "#79B8F5", now).unwrap();
        let b = db::projects::create_project(&conn, "B", "#78DCC0", now).unwrap();
        let item = create_item(
            &conn,
            NewItem {
                project_id: Some(a.id.clone()),
                ..task("moving")
            },
            now,
        )
        .unwrap();
        let to_b = ItemPatch {
            project_id: Some(Some(b.id.clone())),
            ..Default::default()
        };
        update_item(&conn, &item.id, to_b, now, UTC_TZ).unwrap();
        assert!(
            list_items(&conn, &ItemFilter::Board(a.board_id.clone()), false)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            list_items(&conn, &ItemFilter::Board(b.board_id.clone()), false)
                .unwrap()
                .len(),
            1
        );
        let clear = ItemPatch {
            project_id: Some(None),
            ..Default::default()
        };
        update_item(&conn, &item.id, clear, now, UTC_TZ).unwrap();
        assert_eq!(
            ids(&list_items(&conn, &ItemFilter::Inbox, false).unwrap()),
            vec![item.id.as_str()]
        );
    }

    #[test]
    fn complete_sets_done_and_is_idempotent() {
        let conn = db::open_in_memory().unwrap();
        let now = utc(2026, 11, 9, 11, 0);
        let item = create_item(&conn, task("Do it"), now).unwrap();
        let done = complete_item(&conn, &item.id, now, UTC_TZ).unwrap();
        assert_eq!(done.status, Status::Done);
        assert_eq!(done.completed_at, Some(now));
        let again = complete_item(&conn, &item.id, now + Duration::hours(1), UTC_TZ).unwrap();
        assert_eq!(again.completed_at, Some(now));
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
        let next = complete_item(&conn, &item.id, utc(2026, 11, 9, 10, 0), UTC_TZ).unwrap();
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
    fn complete_recurring_count_three_ends_after_three() {
        let conn = db::open_in_memory().unwrap();
        let due = utc(2026, 11, 9, 9, 0);
        let item = create_item(
            &conn,
            NewItem {
                due_at: Some(due),
                recurrence: Some("FREQ=DAILY;COUNT=3".into()),
                ..task("Three times")
            },
            due,
        )
        .unwrap();
        let first = complete_item(&conn, &item.id, due, UTC_TZ).unwrap();
        assert_eq!(first.due_at, Some(utc(2026, 11, 10, 9, 0)));
        let second = complete_item(&conn, &item.id, due, UTC_TZ).unwrap();
        assert_eq!(second.due_at, Some(utc(2026, 11, 11, 9, 0)));
        let third = complete_item(&conn, &item.id, due, UTC_TZ).unwrap();
        assert_eq!(third.status, Status::Done);
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
        let done = complete_item(&conn, &item.id, utc(2026, 11, 9, 10, 0), UTC_TZ).unwrap();
        assert_eq!(done.status, Status::Done);
        assert!(done.completed_at.is_some());
    }

    #[test]
    fn status_done_patch_advances_recurring_item() {
        let conn = db::open_in_memory().unwrap();
        let due = utc(2026, 11, 9, 9, 0);
        let item = create_item(
            &conn,
            NewItem {
                due_at: Some(due),
                recurrence: Some("FREQ=DAILY".into()),
                ..task("Daily")
            },
            due,
        )
        .unwrap();
        let patch = ItemPatch {
            status: Some(Status::Done),
            ..Default::default()
        };
        let next = update_item(&conn, &item.id, patch, due, UTC_TZ).unwrap();
        assert_eq!(next.status, Status::Inbox);
        assert_eq!(next.due_at, Some(utc(2026, 11, 10, 9, 0)));
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
    fn duplicate_copies_fields_and_offsets_position() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let board = db::boards::create_board(&conn, "B", now).unwrap();
        let item = create_item(
            &conn,
            NewItem {
                board_id: Some(board.id.clone()),
                x: Some(100.0),
                y: Some(100.0),
                tags: Some(vec!["x".into()]),
                ..task("copy me")
            },
            now,
        )
        .unwrap();
        let copy = duplicate_item(&conn, &item.id, now).unwrap();
        assert_ne!(copy.id, item.id);
        assert_eq!(copy.title, "copy me");
        assert_eq!(copy.tags, vec!["x".to_string()]);
        let placements = db::placements::list_placements(&conn, &board.id).unwrap();
        let p = placements.iter().find(|p| p.item_id == copy.id).unwrap();
        assert_eq!((p.x, p.y), (124.0, 124.0));
    }

    #[test]
    fn fts_finds_by_body_word_and_follows_updates() {
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
        assert_eq!(
            ids(&search_items(&conn, "remind", 10).unwrap()),
            vec![hit.id.as_str()]
        );
        assert!(search_items(&conn, "", 10).unwrap().is_empty());
        let patch = ItemPatch {
            title: Some("Giulia".into()),
            ..Default::default()
        };
        update_item(&conn, &hit.id, patch, now, UTC_TZ).unwrap();
        assert_eq!(search_items(&conn, "giul", 10).unwrap().len(), 1);
        delete_item(&conn, &hit.id).unwrap();
        assert!(search_items(&conn, "giul", 10).unwrap().is_empty());
    }

    #[test]
    fn counts_and_progress_by_project() {
        let conn = db::open_in_memory().unwrap();
        let now = utc(2026, 11, 9, 11, 0);
        let p = db::projects::create_project(&conn, "P", "#79B8F5", now).unwrap();
        let a = create_item(
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
        complete_item(&conn, &a.id, now, UTC_TZ).unwrap();
        let c = counts(&conn, now, UTC_TZ).unwrap();
        assert_eq!(c.by_project.get(&p.id), Some(&1));
        assert_eq!(c.ideas, 1);
        assert_eq!(c.inbox, 1);
        let progress = project_progress(&conn).unwrap();
        assert_eq!(progress.get(&p.id), Some(&Progress { done: 1, total: 2 }));
    }

    #[test]
    fn hint_roundtrips_and_editing_the_date_resolves_it() {
        let conn = db::open_in_memory().unwrap();
        let now = utc(2026, 11, 9, 11, 0);
        let item = create_item(
            &conn,
            NewItem {
                hint: Some(CaptureHint {
                    date_phrase: Some("the first".into()),
                    low_confidence: false,
                    suggested_project: Some("Kitchen".into()),
                }),
                ..task("Pay rent")
            },
            now,
        )
        .unwrap();
        assert_eq!(
            item.hint.as_ref().unwrap().date_phrase.as_deref(),
            Some("the first")
        );
        let patch = ItemPatch {
            due_at: Some(Some(utc(2026, 12, 1, 9, 0))),
            ..Default::default()
        };
        let after = update_item(&conn, &item.id, patch, now, UTC_TZ).unwrap();
        let hint = after.hint.unwrap();
        assert_eq!(hint.date_phrase, None);
        assert_eq!(hint.suggested_project.as_deref(), Some("Kitchen"));
        let cleared: ItemPatch = serde_json::from_str(r#"{"hint":null}"#).unwrap();
        assert_eq!(
            update_item(&conn, &item.id, cleared, now, UTC_TZ)
                .unwrap()
                .hint,
            None
        );
    }

    #[test]
    fn local_midnight_steps_over_dst_gap() {
        // Santiago starts DST at 00:00 on 2026-09-06 (00:00 → 01:00).
        let tz: Tz = "America/Santiago".parse().unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 9, 6).unwrap();
        assert!(local_midnight(date, tz).is_some());
    }
}
