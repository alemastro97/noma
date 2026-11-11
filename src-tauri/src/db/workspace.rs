//! Workspace power tools: saved filters, canvas group frames, bulk moves that
//! report where cards were (for undo), and row snapshots so a deleted card can
//! be restored with the same id.

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use rusqlite::{Connection, OptionalExtension, Row, params, types::Value as SqlValue};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::error::{DbError, DbResult};
use super::filing::{self, Place};
use super::models::{Link, new_id};

// ── Saved filters ───────────────────────────────────────────────────────────

/// A filter the user saved as a sidebar entry. `spec` is owned by the frontend
/// (types, tags, projects, statuses, date range, text) and stored as JSON.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SavedFilter {
    pub id: String,
    pub name: String,
    pub spec: Value,
    pub created_at: DateTime<Utc>,
}

fn row_to_filter(row: &Row<'_>) -> rusqlite::Result<SavedFilter> {
    let spec: String = row.get("spec")?;
    Ok(SavedFilter {
        id: row.get("id")?,
        name: row.get("name")?,
        spec: serde_json::from_str(&spec).unwrap_or(Value::Null),
        created_at: row.get("created_at")?,
    })
}

pub fn list_filters(conn: &Connection) -> DbResult<Vec<SavedFilter>> {
    let mut stmt = conn.prepare("SELECT * FROM saved_filters ORDER BY created_at, id")?;
    let rows = stmt.query_map([], row_to_filter)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn create_filter(
    conn: &Connection,
    name: &str,
    spec: &Value,
    now: DateTime<Utc>,
) -> DbResult<SavedFilter> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DbError::Invalid("filter name must not be blank".into()));
    }
    let filter = SavedFilter {
        id: new_id(),
        name: name.to_string(),
        spec: spec.clone(),
        created_at: now,
    };
    conn.execute(
        "INSERT INTO saved_filters (id, name, spec, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![filter.id, filter.name, spec.to_string(), filter.created_at],
    )?;
    Ok(filter)
}

pub fn delete_filter(conn: &Connection, id: &str) -> DbResult<()> {
    conn.execute("DELETE FROM saved_filters WHERE id = ?1", [id])?;
    Ok(())
}

// ── Canvas groups ───────────────────────────────────────────────────────────

/// A labelled frame on a board. Cards whose box sits inside it move with it
/// (computed by the canvas; membership is not stored).
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CanvasGroup {
    pub id: String,
    pub board_id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Frame geometry and label; every field is optional on update.
#[derive(Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GroupPatch {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    pub w: Option<f64>,
    #[serde(default)]
    pub h: Option<f64>,
}

const MIN_GROUP: f64 = 80.0;

fn row_to_group(row: &Row<'_>) -> rusqlite::Result<CanvasGroup> {
    Ok(CanvasGroup {
        id: row.get("id")?,
        board_id: row.get("board_id")?,
        label: row.get("label")?,
        x: row.get("x")?,
        y: row.get("y")?,
        w: row.get("w")?,
        h: row.get("h")?,
    })
}

pub fn list_groups(conn: &Connection, board_id: &str) -> DbResult<Vec<CanvasGroup>> {
    let mut stmt =
        conn.prepare("SELECT * FROM canvas_groups WHERE board_id = ?1 ORDER BY created_at, id")?;
    let rows = stmt.query_map([board_id], row_to_group)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn get_group(conn: &Connection, id: &str) -> DbResult<Option<CanvasGroup>> {
    Ok(conn
        .query_row(
            "SELECT * FROM canvas_groups WHERE id = ?1",
            [id],
            row_to_group,
        )
        .optional()?)
}

#[allow(clippy::too_many_arguments)]
pub fn create_group(
    conn: &Connection,
    board_id: &str,
    label: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    now: DateTime<Utc>,
) -> DbResult<CanvasGroup> {
    let group = CanvasGroup {
        id: new_id(),
        board_id: board_id.to_string(),
        label: label.trim().to_string(),
        x,
        y,
        w: w.max(MIN_GROUP),
        h: h.max(MIN_GROUP),
    };
    conn.execute(
        "INSERT INTO canvas_groups (id, board_id, label, x, y, w, h, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            group.id,
            group.board_id,
            group.label,
            group.x,
            group.y,
            group.w,
            group.h,
            now
        ],
    )?;
    Ok(group)
}

pub fn update_group(conn: &Connection, id: &str, patch: GroupPatch) -> DbResult<CanvasGroup> {
    let mut g = get_group(conn, id)?.ok_or_else(|| DbError::NotFound(format!("group {id}")))?;
    if let Some(label) = patch.label {
        g.label = label.trim().to_string();
    }
    g.x = patch.x.unwrap_or(g.x);
    g.y = patch.y.unwrap_or(g.y);
    g.w = patch.w.unwrap_or(g.w).max(MIN_GROUP);
    g.h = patch.h.unwrap_or(g.h).max(MIN_GROUP);
    conn.execute(
        "UPDATE canvas_groups SET label = ?2, x = ?3, y = ?4, w = ?5, h = ?6 WHERE id = ?1",
        params![g.id, g.label, g.x, g.y, g.w, g.h],
    )?;
    Ok(g)
}

pub fn delete_group(conn: &Connection, id: &str) -> DbResult<()> {
    conn.execute("DELETE FROM canvas_groups WHERE id = ?1", [id])?;
    Ok(())
}

// ── Links graph ─────────────────────────────────────────────────────────────

/// Every link (personal scale: a few thousand at most). The graph view keeps
/// the ones whose both ends are on screen.
pub fn list_all_links(conn: &Connection) -> DbResult<Vec<Link>> {
    let mut stmt = conn.prepare("SELECT * FROM connections ORDER BY created_at, id")?;
    let rows = stmt.query_map([], |row| {
        Ok(Link {
            id: row.get("id")?,
            from_item: row.get("from_item")?,
            to_item: row.get("to_item")?,
            label: row.get("label")?,
            created_at: row.get("created_at")?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

// ── Bulk move ───────────────────────────────────────────────────────────────

/// Where a card was before a move, so undo can put it back (and at the same
/// canvas spot when it was on a board).
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PriorPlace {
    pub id: String,
    pub place: Place,
    pub board_id: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
}

fn placement_of(conn: &Connection, item_id: &str) -> DbResult<Option<(String, f64, f64)>> {
    Ok(conn
        .query_row(
            "SELECT board_id, x, y FROM placements WHERE item_id = ?1",
            [item_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?)
}

/// Moves every card to `place` in one transaction and returns where each was.
pub fn move_items(
    conn: &Connection,
    ids: &[String],
    place: &Place,
    now: DateTime<Utc>,
    tz: Tz,
) -> DbResult<Vec<PriorPlace>> {
    let tx = conn.unchecked_transaction()?;
    let mut prior = Vec::with_capacity(ids.len());
    for id in ids {
        let (was, _) = filing::place_of(&tx, id)?;
        let at = placement_of(&tx, id)?;
        prior.push(PriorPlace {
            id: id.clone(),
            place: was,
            board_id: at.as_ref().map(|a| a.0.clone()),
            x: at.as_ref().map(|a| a.1),
            y: at.as_ref().map(|a| a.2),
        });
        filing::move_item(&tx, id, place, now, tz)?;
    }
    tx.commit()?;
    Ok(prior)
}

/// Puts cards back where `prior` says they were (the inverse of `move_items`).
pub fn restore_places(
    conn: &Connection,
    prior: &[PriorPlace],
    now: DateTime<Utc>,
    tz: Tz,
) -> DbResult<()> {
    let tx = conn.unchecked_transaction()?;
    for p in prior {
        filing::move_item(&tx, &p.id, &p.place, now, tz)?;
        if let (Some(board), Some(x), Some(y)) = (&p.board_id, p.x, p.y) {
            super::placements::place_item(&tx, &p.id, board, x, y)?;
        }
    }
    tx.commit()?;
    Ok(())
}

// ── Snapshots (undo a committed delete) ─────────────────────────────────────

/// Rows that belong to a card, captured before it is deleted.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub item: Map<String, Value>,
    #[serde(default)]
    pub tags: Vec<Map<String, Value>>,
    #[serde(default)]
    pub placements: Vec<Map<String, Value>>,
    #[serde(default)]
    pub connections: Vec<Map<String, Value>>,
}

fn sql_to_json(v: SqlValue) -> Value {
    match v {
        SqlValue::Null => Value::Null,
        SqlValue::Integer(i) => Value::from(i),
        SqlValue::Real(f) => Value::from(f),
        SqlValue::Text(s) => Value::from(s),
        // No blob columns on these tables; keep the shape anyway.
        SqlValue::Blob(b) => Value::from(b),
    }
}

fn json_to_sql(v: &Value) -> SqlValue {
    match v {
        Value::Null => SqlValue::Null,
        Value::Bool(b) => SqlValue::Integer(i64::from(*b)),
        Value::Number(n) => n
            .as_i64()
            .map(SqlValue::Integer)
            .unwrap_or_else(|| SqlValue::Real(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => SqlValue::Text(s.clone()),
        other => SqlValue::Text(other.to_string()),
    }
}

fn rows_as_maps(conn: &Connection, sql: &str, id: &str) -> DbResult<Vec<Map<String, Value>>> {
    let mut stmt = conn.prepare(sql)?;
    let names: Vec<String> = stmt.column_names().map(str::to_string).collect();
    let rows = stmt.query_map([id], |row| {
        let mut m = Map::new();
        for (i, name) in names.iter().enumerate() {
            m.insert(name.clone(), sql_to_json(row.get::<_, SqlValue>(i)?));
        }
        Ok(m)
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Captures each existing card's rows. Missing ids are skipped.
pub fn snapshot(conn: &Connection, ids: &[String]) -> DbResult<Vec<Snapshot>> {
    let mut out = Vec::new();
    for id in ids {
        let Some(item) = rows_as_maps(conn, "SELECT * FROM items WHERE id = ?1", id)?
            .into_iter()
            .next()
        else {
            continue;
        };
        out.push(Snapshot {
            item,
            tags: rows_as_maps(conn, "SELECT * FROM item_tags WHERE item_id = ?1", id)?,
            placements: rows_as_maps(conn, "SELECT * FROM placements WHERE item_id = ?1", id)?,
            connections: rows_as_maps(
                conn,
                "SELECT * FROM connections WHERE from_item = ?1 OR to_item = ?1",
                id,
            )?,
        });
    }
    Ok(out)
}

/// Column names come from our own snapshot of `table`; only plain identifiers
/// are accepted so a tampered snapshot cannot inject SQL.
fn insert_row(conn: &Connection, table: &str, row: &Map<String, Value>) -> DbResult<usize> {
    if row.is_empty()
        || !row
            .keys()
            .all(|k| !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
    {
        return Err(DbError::Invalid("malformed snapshot".into()));
    }
    let cols: Vec<&str> = row.keys().map(String::as_str).collect();
    let marks: Vec<String> = (1..=cols.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "INSERT OR IGNORE INTO {table} ({}) VALUES ({})",
        cols.join(", "),
        marks.join(", ")
    );
    let values: Vec<SqlValue> = row.values().map(json_to_sql).collect();
    Ok(conn.execute(&sql, rusqlite::params_from_iter(values))?)
}

fn exists(conn: &Connection, table: &str, id: &Value) -> DbResult<bool> {
    let Some(id) = id.as_str() else {
        return Ok(false);
    };
    let sql = format!("SELECT EXISTS (SELECT 1 FROM {table} WHERE id = ?1)");
    Ok(conn.query_row(&sql, [id], |r| r.get(0))?)
}

/// Re-inserts deleted cards with their ids, tags, canvas spot and links (links
/// to cards that are gone are skipped). Returns the restored ids.
pub fn restore(conn: &Connection, snapshots: &[Snapshot]) -> DbResult<Vec<String>> {
    let tx = conn.unchecked_transaction()?;
    let mut restored = Vec::new();
    // Items first, so links between two restored cards find both ends.
    for s in snapshots {
        // A project that was deleted meanwhile would break the foreign key.
        let mut item = s.item.clone();
        if let Some(p) = item.get("project_id").cloned()
            && !p.is_null()
            && !exists(&tx, "projects", &p)?
        {
            item.insert("project_id".into(), Value::Null);
        }
        if insert_row(&tx, "items", &item)? > 0
            && let Some(id) = item.get("id").and_then(Value::as_str)
        {
            restored.push(id.to_string());
        }
    }
    for s in snapshots {
        for t in &s.tags {
            if t.get("tag_id")
                .is_some_and(|v| exists(&tx, "tags", v).unwrap_or(false))
            {
                insert_row(&tx, "item_tags", t)?;
            }
        }
        for p in &s.placements {
            if p.get("board_id")
                .is_some_and(|v| exists(&tx, "boards", v).unwrap_or(false))
            {
                insert_row(&tx, "placements", p)?;
            }
        }
        for c in &s.connections {
            let ends = ["from_item", "to_item"].iter().all(|k| {
                c.get(*k)
                    .is_some_and(|v| exists(&tx, "items", v).unwrap_or(false))
            });
            if ends {
                insert_row(&tx, "connections", c)?;
            }
        }
    }
    tx.commit()?;
    Ok(restored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{self, models::*};
    use chrono::TimeZone;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 11, 9, 0, 0).unwrap()
    }

    fn item(conn: &Connection, title: &str) -> Item {
        db::items::create_item(
            conn,
            NewItem {
                kind: ItemType::Task,
                title: title.into(),
                tags: Some(vec!["work".into()]),
                ..Default::default()
            },
            now(),
        )
        .unwrap()
    }

    #[test]
    fn snapshot_restores_item_tags_and_links() {
        let conn = db::open_in_memory().unwrap();
        let a = item(&conn, "A");
        let b = item(&conn, "B");
        db::links::create_link(&conn, &a.id, &b.id, None, now()).unwrap();
        let snap = snapshot(&conn, std::slice::from_ref(&a.id)).unwrap();
        db::items::delete_item(&conn, &a.id).unwrap();
        assert!(db::items::get_item(&conn, &a.id).unwrap().is_none());
        let back = restore(&conn, &snap).unwrap();
        assert_eq!(back, vec![a.id.clone()]);
        let again = db::items::get_item(&conn, &a.id).unwrap().unwrap();
        assert_eq!(again.title, "A");
        assert_eq!(again.tags, vec!["work".to_string()]);
        assert_eq!(again.link_count, 1);
    }

    #[test]
    fn move_items_reports_prior_place() {
        let conn = db::open_in_memory().unwrap();
        let a = item(&conn, "A");
        let board = db::boards::create_board(&conn, "Ideas", now()).unwrap();
        let prior = move_items(
            &conn,
            std::slice::from_ref(&a.id),
            &Place::Board(board.id.clone()),
            now(),
            chrono_tz::UTC,
        )
        .unwrap();
        assert_eq!(prior[0].place, Place::Inbox);
        restore_places(&conn, &prior, now(), chrono_tz::UTC).unwrap();
        assert_eq!(filing::place_of(&conn, &a.id).unwrap().0, Place::Inbox);
    }

    #[test]
    fn groups_and_filters_round_trip() {
        let conn = db::open_in_memory().unwrap();
        let board = db::boards::create_board(&conn, "B", now()).unwrap();
        let g = create_group(&conn, &board.id, "Week", 0.0, 0.0, 10.0, 10.0, now()).unwrap();
        assert_eq!(g.w, MIN_GROUP);
        let g = update_group(
            &conn,
            &g.id,
            GroupPatch {
                x: Some(40.0),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(list_groups(&conn, &board.id).unwrap(), vec![g.clone()]);
        delete_group(&conn, &g.id).unwrap();
        assert!(list_groups(&conn, &board.id).unwrap().is_empty());

        let f = create_filter(
            &conn,
            " Work ",
            &serde_json::json!({"tags": ["work"]}),
            now(),
        )
        .unwrap();
        assert_eq!(f.name, "Work");
        assert_eq!(list_filters(&conn).unwrap(), vec![f.clone()]);
        delete_filter(&conn, &f.id).unwrap();
        assert!(list_filters(&conn).unwrap().is_empty());
    }
}
