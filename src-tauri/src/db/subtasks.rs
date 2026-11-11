//! Checklist inside a card: ordered, checkable subtasks.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use super::error::{DbError, DbResult};
use super::models::new_id;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Subtask {
    pub id: String,
    pub item_id: String,
    pub title: String,
    pub done: bool,
    pub position: i64,
    pub created_at: DateTime<Utc>,
}

fn row_to_subtask(row: &Row<'_>) -> rusqlite::Result<Subtask> {
    Ok(Subtask {
        id: row.get("id")?,
        item_id: row.get("item_id")?,
        title: row.get("title")?,
        done: row.get("done")?,
        position: row.get("position")?,
        created_at: row.get("created_at")?,
    })
}

fn clean_title(title: &str) -> DbResult<String> {
    let title = title.trim();
    if title.is_empty() {
        return Err(DbError::Invalid("subtask must not be blank".into()));
    }
    Ok(title.to_string())
}

pub fn list_subtasks(conn: &Connection, item_id: &str) -> DbResult<Vec<Subtask>> {
    let mut stmt = conn.prepare_cached(
        "SELECT * FROM subtasks WHERE item_id = ?1 ORDER BY position ASC, created_at ASC",
    )?;
    let rows = stmt.query_map([item_id], row_to_subtask)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn get_subtask(conn: &Connection, id: &str) -> DbResult<Subtask> {
    conn.query_row("SELECT * FROM subtasks WHERE id = ?1", [id], row_to_subtask)
        .optional()?
        .ok_or_else(|| DbError::NotFound(format!("subtask {id}")))
}

/// Appends a subtask at the end of the card's checklist.
pub fn add_subtask(
    conn: &Connection,
    item_id: &str,
    title: &str,
    now: DateTime<Utc>,
) -> DbResult<Subtask> {
    let title = clean_title(title)?;
    let position: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM subtasks WHERE item_id = ?1",
        [item_id],
        |r| r.get(0),
    )?;
    let id = new_id();
    conn.execute(
        "INSERT INTO subtasks (id, item_id, title, done, position, created_at) \
         VALUES (?1, ?2, ?3, 0, ?4, ?5)",
        params![id, item_id, title, position, now],
    )?;
    get_subtask(conn, &id)
}

/// Renames and/or checks a subtask. `None` leaves a field alone.
pub fn update_subtask(
    conn: &Connection,
    id: &str,
    title: Option<&str>,
    done: Option<bool>,
) -> DbResult<Subtask> {
    if let Some(title) = title {
        let title = clean_title(title)?;
        conn.execute(
            "UPDATE subtasks SET title = ?2 WHERE id = ?1",
            params![id, title],
        )?;
    }
    if let Some(done) = done {
        conn.execute(
            "UPDATE subtasks SET done = ?2 WHERE id = ?1",
            params![id, done],
        )?;
    }
    get_subtask(conn, id)
}

/// Deletes a subtask and returns the card it belonged to.
pub fn delete_subtask(conn: &Connection, id: &str) -> DbResult<String> {
    let subtask = get_subtask(conn, id)?;
    conn.execute("DELETE FROM subtasks WHERE id = ?1", [id])?;
    Ok(subtask.item_id)
}

/// Rewrites positions to follow `ids`; ids of other cards are ignored.
pub fn reorder_subtasks(conn: &Connection, item_id: &str, ids: &[String]) -> DbResult<()> {
    let tx = conn.unchecked_transaction()?;
    for (position, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE subtasks SET position = ?3 WHERE id = ?1 AND item_id = ?2",
            params![id, item_id, position as i64],
        )?;
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{
        self,
        items::{create_item, get_item},
        models::{ItemType, NewItem},
    };

    #[test]
    fn add_check_reorder_and_count() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let item = create_item(
            &conn,
            NewItem {
                kind: ItemType::Task,
                title: "Trip".into(),
                ..Default::default()
            },
            now,
        )
        .unwrap();
        let a = add_subtask(&conn, &item.id, "Passport", now).unwrap();
        let b = add_subtask(&conn, &item.id, "Tickets", now).unwrap();
        assert!(add_subtask(&conn, &item.id, "  ", now).is_err());
        update_subtask(&conn, &a.id, None, Some(true)).unwrap();
        reorder_subtasks(&conn, &item.id, &[b.id.clone(), a.id.clone()]).unwrap();
        let list = list_subtasks(&conn, &item.id).unwrap();
        assert_eq!(list[0].id, b.id);
        let card = get_item(&conn, &item.id).unwrap().unwrap();
        assert_eq!((card.subtasks_done, card.subtasks_total), (1, 2));
        assert_eq!(delete_subtask(&conn, &a.id).unwrap(), item.id);
    }
}
