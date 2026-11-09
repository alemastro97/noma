use rusqlite::{Connection, Row, params};

use super::error::DbResult;
use super::models::Placement;

fn row_to_placement(row: &Row<'_>) -> rusqlite::Result<Placement> {
    Ok(Placement {
        item_id: row.get("item_id")?,
        board_id: row.get("board_id")?,
        x: row.get("x")?,
        y: row.get("y")?,
    })
}

/// Places (or moves) an item on a board. An item lives on at most one board in v1.
pub fn place_item(
    conn: &Connection,
    item_id: &str,
    board_id: &str,
    x: f64,
    y: f64,
) -> DbResult<Placement> {
    conn.execute(
        "INSERT INTO placements (item_id, board_id, x, y) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(item_id) DO UPDATE SET board_id = excluded.board_id, x = excluded.x, y = excluded.y",
        params![item_id, board_id, x, y],
    )?;
    Ok(Placement {
        item_id: item_id.to_string(),
        board_id: board_id.to_string(),
        x,
        y,
    })
}

pub fn remove_placement(conn: &Connection, item_id: &str) -> DbResult<()> {
    conn.execute("DELETE FROM placements WHERE item_id = ?1", [item_id])?;
    Ok(())
}

pub fn list_placements(conn: &Connection, board_id: &str) -> DbResult<Vec<Placement>> {
    let mut stmt = conn.prepare("SELECT * FROM placements WHERE board_id = ?1")?;
    let rows = stmt.query_map([board_id], row_to_placement)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{self, models::*};
    use chrono::Utc;

    #[test]
    fn place_then_move_keeps_one_row() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let board = db::boards::create_board(&conn, "B", now).unwrap();
        let item = db::items::create_item(
            &conn,
            NewItem {
                kind: ItemType::Task,
                title: "T".into(),
                ..Default::default()
            },
            now,
        )
        .unwrap();
        place_item(&conn, &item.id, &board.id, 10.0, 20.0).unwrap();
        place_item(&conn, &item.id, &board.id, 30.0, 40.0).unwrap();
        let all = list_placements(&conn, &board.id).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!((all[0].x, all[0].y), (30.0, 40.0));
    }
}
