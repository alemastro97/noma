use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row, params};

use super::error::{DbError, DbResult};
use super::models::{Link, new_id};

fn row_to_link(row: &Row<'_>) -> rusqlite::Result<Link> {
    Ok(Link {
        id: row.get("id")?,
        from_item: row.get("from_item")?,
        to_item: row.get("to_item")?,
        label: row.get("label")?,
        created_at: row.get("created_at")?,
    })
}

/// Connects two items. Linking an item to itself is rejected; linking the same
/// pair twice (in either direction) returns the existing link.
pub fn create_link(
    conn: &Connection,
    from_item: &str,
    to_item: &str,
    label: Option<&str>,
    now: DateTime<Utc>,
) -> DbResult<Link> {
    if from_item == to_item {
        return Err(DbError::Invalid(
            "an item cannot be linked to itself".into(),
        ));
    }
    let mut stmt = conn.prepare(
        "SELECT * FROM connections WHERE (from_item = ?1 AND to_item = ?2) OR (from_item = ?2 AND to_item = ?1)",
    )?;
    if let Some(existing) = stmt
        .query_map(params![from_item, to_item], row_to_link)?
        .next()
    {
        return Ok(existing?);
    }
    let link = Link {
        id: new_id(),
        from_item: from_item.to_string(),
        to_item: to_item.to_string(),
        label: label.map(str::to_string),
        created_at: now,
    };
    conn.execute(
        "INSERT INTO connections (id, from_item, to_item, label, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![link.id, link.from_item, link.to_item, link.label, link.created_at],
    )?;
    Ok(link)
}

/// Links whose both ends are placed on `board_id`.
pub fn list_links_for_board(conn: &Connection, board_id: &str) -> DbResult<Vec<Link>> {
    let mut stmt = conn.prepare(
        "SELECT c.* FROM connections c
         JOIN placements a ON a.item_id = c.from_item AND a.board_id = ?1
         JOIN placements b ON b.item_id = c.to_item AND b.board_id = ?1
         ORDER BY c.created_at",
    )?;
    let rows = stmt.query_map([board_id], row_to_link)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn delete_link(conn: &Connection, id: &str) -> DbResult<()> {
    let changed = conn.execute("DELETE FROM connections WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(DbError::NotFound(format!("link {id}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{self, models::*};

    fn item(conn: &Connection, board: &str, title: &str) -> Item {
        db::items::create_item(
            conn,
            NewItem {
                title: title.into(),
                board_id: Some(board.into()),
                ..Default::default()
            },
            Utc::now(),
        )
        .unwrap()
    }

    #[test]
    fn create_list_dedupe_and_cascade() {
        let conn = db::open_in_memory().unwrap();
        let board = db::boards::create_board(&conn, "B", Utc::now()).unwrap();
        let a = item(&conn, &board.id, "a");
        let b = item(&conn, &board.id, "b");
        let l1 = create_link(&conn, &a.id, &b.id, None, Utc::now()).unwrap();
        let l2 = create_link(&conn, &b.id, &a.id, None, Utc::now()).unwrap();
        assert_eq!(l1.id, l2.id);
        assert_eq!(list_links_for_board(&conn, &board.id).unwrap().len(), 1);
        assert_eq!(
            db::items::get_item(&conn, &a.id)
                .unwrap()
                .unwrap()
                .link_count,
            1
        );
        db::items::delete_item(&conn, &b.id).unwrap();
        assert!(list_links_for_board(&conn, &board.id).unwrap().is_empty());
    }

    #[test]
    fn self_link_rejected() {
        let conn = db::open_in_memory().unwrap();
        let board = db::boards::create_board(&conn, "B", Utc::now()).unwrap();
        let a = item(&conn, &board.id, "a");
        assert!(create_link(&conn, &a.id, &a.id, None, Utc::now()).is_err());
    }

    #[test]
    fn delete_missing_link_is_not_found() {
        let conn = db::open_in_memory().unwrap();
        assert!(matches!(
            delete_link(&conn, "nope"),
            Err(DbError::NotFound(_))
        ));
    }
}
