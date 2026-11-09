use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row, params};

use super::error::{DbError, DbResult};
use super::models::{Board, BoardKind, new_id};

fn row_to_board(row: &Row<'_>) -> rusqlite::Result<Board> {
    Ok(Board {
        id: row.get("id")?,
        name: row.get("name")?,
        kind: row.get("kind")?,
        created_at: row.get("created_at")?,
    })
}

pub(crate) fn insert_board(
    conn: &Connection,
    name: &str,
    kind: BoardKind,
    now: DateTime<Utc>,
) -> DbResult<Board> {
    let board = Board {
        id: new_id(),
        name: name.trim().to_string(),
        kind,
        created_at: now,
    };
    if board.name.is_empty() {
        return Err(DbError::Invalid("board name must not be blank".into()));
    }
    conn.execute(
        "INSERT INTO boards (id, name, kind, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![board.id, board.name, board.kind, board.created_at],
    )?;
    Ok(board)
}

/// Creates a personal board.
pub fn create_board(conn: &Connection, name: &str, now: DateTime<Utc>) -> DbResult<Board> {
    insert_board(conn, name, BoardKind::Personal, now)
}

pub fn list_boards(conn: &Connection) -> DbResult<Vec<Board>> {
    let mut stmt = conn.prepare("SELECT * FROM boards ORDER BY created_at ASC")?;
    let rows = stmt.query_map([], row_to_board)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn get_board(conn: &Connection, id: &str) -> DbResult<Option<Board>> {
    let mut stmt = conn.prepare("SELECT * FROM boards WHERE id = ?1")?;
    let mut rows = stmt.query_map([id], row_to_board)?;
    rows.next().transpose().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn create_and_list_personal_boards() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let b = create_board(&conn, "Thinking space", now).unwrap();
        assert_eq!(b.kind, BoardKind::Personal);
        let all = list_boards(&conn).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "Thinking space");
        assert_eq!(get_board(&conn, &b.id).unwrap().unwrap().id, b.id);
    }

    #[test]
    fn blank_name_is_rejected() {
        let conn = db::open_in_memory().unwrap();
        assert!(create_board(&conn, "   ", Utc::now()).is_err());
    }
}
