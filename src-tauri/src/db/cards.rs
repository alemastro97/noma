//! Card lifecycle beyond editing: pinning, the Archive and the Trash.
//!
//! Archived and trashed cards keep their row (and placement, subtasks and
//! attachments) and are hidden from every view by `items::VISIBLE`. Trashed
//! cards are purged for good after `TRASH_DAYS`.

use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, params};

use super::error::{DbError, DbResult};
use super::items::{SELECT_ITEM, get_item, query_items};
use super::models::Item;

/// How long a card stays in the Trash before it is deleted for good.
pub const TRASH_DAYS: i64 = 30;

fn require_changed(changed: usize, id: &str) -> DbResult<()> {
    if changed == 0 {
        return Err(DbError::NotFound(format!("item {id}")));
    }
    Ok(())
}

fn fetch(conn: &Connection, id: &str) -> DbResult<Item> {
    get_item(conn, id)?.ok_or_else(|| DbError::NotFound(format!("item {id}")))
}

pub fn set_pinned(conn: &Connection, id: &str, pinned: bool) -> DbResult<Item> {
    let changed = conn.execute(
        "UPDATE items SET pinned = ?2 WHERE id = ?1",
        params![id, pinned],
    )?;
    require_changed(changed, id)?;
    fetch(conn, id)
}

/// Archives (`true`) or restores (`false`) a card.
pub fn set_archived(
    conn: &Connection,
    id: &str,
    archived: bool,
    now: DateTime<Utc>,
) -> DbResult<Item> {
    let changed = conn.execute(
        "UPDATE items SET archived_at = ?2 WHERE id = ?1",
        params![id, archived.then_some(now)],
    )?;
    require_changed(changed, id)?;
    fetch(conn, id)
}

/// Moves a card to the Trash.
pub fn trash(conn: &Connection, id: &str, now: DateTime<Utc>) -> DbResult<Item> {
    let changed = conn.execute(
        "UPDATE items SET deleted_at = ?2 WHERE id = ?1",
        params![id, now],
    )?;
    require_changed(changed, id)?;
    fetch(conn, id)
}

/// Takes a card out of the Trash (it returns to the Archive if it was archived).
pub fn restore(conn: &Connection, id: &str) -> DbResult<Item> {
    let changed = conn.execute("UPDATE items SET deleted_at = NULL WHERE id = ?1", [id])?;
    require_changed(changed, id)?;
    fetch(conn, id)
}

/// Archived cards (not in the Trash), most recently archived first.
pub fn list_archived(conn: &Connection) -> DbResult<Vec<Item>> {
    let sql = format!(
        "{SELECT_ITEM} WHERE i.archived_at IS NOT NULL AND i.deleted_at IS NULL \
         ORDER BY i.archived_at DESC"
    );
    query_items(conn, &sql, &[])
}

/// Cards in the Trash, most recently deleted first.
pub fn list_trashed(conn: &Connection) -> DbResult<Vec<Item>> {
    let sql = format!("{SELECT_ITEM} WHERE i.deleted_at IS NOT NULL ORDER BY i.deleted_at DESC");
    query_items(conn, &sql, &[])
}

fn delete_where(
    conn: &Connection,
    condition: &str,
    p: &[&dyn rusqlite::ToSql],
) -> DbResult<Vec<String>> {
    let tx = conn.unchecked_transaction()?;
    let ids: Vec<String> = {
        let mut stmt = tx.prepare(&format!("SELECT id FROM items WHERE {condition}"))?;
        stmt.query_map(p, |r| r.get(0))?.collect::<Result<_, _>>()?
    };
    for id in &ids {
        tx.execute("DELETE FROM items WHERE id = ?1", [id])?;
    }
    tx.commit()?;
    Ok(ids)
}

/// Deletes one trashed card for good. Returns its id (for attachment cleanup).
pub fn delete_forever(conn: &Connection, id: &str) -> DbResult<Vec<String>> {
    delete_where(conn, "id = ?1 AND deleted_at IS NOT NULL", &[&id])
}

/// Deletes every card in the Trash for good. Returns the deleted ids.
pub fn empty_trash(conn: &Connection) -> DbResult<Vec<String>> {
    delete_where(conn, "deleted_at IS NOT NULL", &[])
}

/// Deletes cards trashed more than `TRASH_DAYS` before `now`. Returns the deleted ids.
pub fn purge_trash(conn: &Connection, now: DateTime<Utc>) -> DbResult<Vec<String>> {
    let cutoff = now - Duration::days(TRASH_DAYS);
    delete_where(
        conn,
        "deleted_at IS NOT NULL AND deleted_at < ?1",
        &[&cutoff],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{
        self,
        items::{create_item, list_items},
        models::{ItemFilter, ItemType, NewItem},
    };

    fn task(conn: &Connection, title: &str) -> Item {
        create_item(
            conn,
            NewItem {
                kind: ItemType::Task,
                title: title.into(),
                ..Default::default()
            },
            Utc::now(),
        )
        .unwrap()
    }

    #[test]
    fn archive_and_trash_hide_cards_until_restored() {
        let conn = db::open_in_memory().unwrap();
        let a = task(&conn, "a");
        let b = task(&conn, "b");
        set_archived(&conn, &a.id, true, Utc::now()).unwrap();
        trash(&conn, &b.id, Utc::now()).unwrap();
        assert!(
            list_items(&conn, &ItemFilter::All, true)
                .unwrap()
                .is_empty()
        );
        assert_eq!(list_archived(&conn).unwrap().len(), 1);
        assert_eq!(list_trashed(&conn).unwrap().len(), 1);
        set_archived(&conn, &a.id, false, Utc::now()).unwrap();
        restore(&conn, &b.id).unwrap();
        assert_eq!(list_items(&conn, &ItemFilter::All, true).unwrap().len(), 2);
    }

    #[test]
    fn pinned_cards_sort_first() {
        let conn = db::open_in_memory().unwrap();
        task(&conn, "a");
        let b = task(&conn, "b");
        set_pinned(&conn, &b.id, true).unwrap();
        let all = list_items(&conn, &ItemFilter::All, false).unwrap();
        assert_eq!(all[0].id, b.id);
        assert!(all[0].pinned);
    }

    #[test]
    fn purge_removes_only_old_trash() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let old = task(&conn, "old");
        let recent = task(&conn, "recent");
        trash(&conn, &old.id, now - Duration::days(31)).unwrap();
        trash(&conn, &recent.id, now - Duration::days(2)).unwrap();
        assert_eq!(purge_trash(&conn, now).unwrap(), vec![old.id]);
        assert_eq!(empty_trash(&conn).unwrap(), vec![recent.id]);
    }
}
