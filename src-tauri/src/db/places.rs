//! Managing projects and personal boards: rename, recolor, archive, delete.
//!
//! Deleting a place never loses cards silently: its cards move to the Trash
//! (restorable for 30 days) before the project or board row goes away.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};

use super::boards::row_to_board;
use super::error::{DbError, DbResult};
use super::models::{Board, BoardKind, Project};
use super::projects::{get_project, row_to_project};

fn clean_name(name: &str) -> DbResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DbError::Invalid("name must not be blank".into()));
    }
    Ok(name.to_string())
}

fn require_project(conn: &Connection, id: &str) -> DbResult<Project> {
    get_project(conn, id)?.ok_or_else(|| DbError::NotFound(format!("project {id}")))
}

/// A personal board (project boards are managed through their project).
fn require_personal_board(conn: &Connection, id: &str) -> DbResult<Board> {
    let board = conn
        .query_row("SELECT * FROM boards WHERE id = ?1", [id], row_to_board)
        .optional()?
        .ok_or_else(|| DbError::NotFound(format!("board {id}")))?;
    if board.kind != BoardKind::Personal {
        return Err(DbError::Invalid(
            "project boards are managed through their project".into(),
        ));
    }
    Ok(board)
}

/// Renames a project and its board.
pub fn rename_project(conn: &Connection, id: &str, name: &str) -> DbResult<Project> {
    let name = clean_name(name)?;
    let project = require_project(conn, id)?;
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE projects SET name = ?2 WHERE id = ?1",
        params![id, name],
    )?;
    tx.execute(
        "UPDATE boards SET name = ?2 WHERE id = ?1",
        params![project.board_id, name],
    )?;
    tx.commit()?;
    require_project(conn, id)
}

pub fn set_project_color(conn: &Connection, id: &str, color: &str) -> DbResult<Project> {
    let color = color.trim();
    let valid = color.len() == 7
        && color.starts_with('#')
        && color[1..].chars().all(|c| c.is_ascii_hexdigit());
    if !valid {
        return Err(DbError::Invalid(format!("color {color:?}")));
    }
    require_project(conn, id)?;
    conn.execute(
        "UPDATE projects SET color = ?2 WHERE id = ?1",
        params![id, color],
    )?;
    require_project(conn, id)
}

/// Archives (`true`) or restores (`false`) a project. Its cards are hidden while archived.
pub fn set_project_archived(
    conn: &Connection,
    id: &str,
    archived: bool,
    now: DateTime<Utc>,
) -> DbResult<Project> {
    require_project(conn, id)?;
    conn.execute(
        "UPDATE projects SET archived_at = ?2 WHERE id = ?1",
        params![id, archived.then_some(now)],
    )?;
    require_project(conn, id)
}

/// Moves the project's cards to the Trash, then deletes the project and its board.
/// Returns how many cards went to the Trash.
pub fn delete_project(conn: &Connection, id: &str, now: DateTime<Utc>) -> DbResult<usize> {
    let project = require_project(conn, id)?;
    let tx = conn.unchecked_transaction()?;
    let trashed = tx.execute(
        "UPDATE items SET deleted_at = ?2 WHERE project_id = ?1 AND deleted_at IS NULL",
        params![id, now],
    )?;
    // items.project_id is ON DELETE SET NULL; placements on the board cascade.
    tx.execute("DELETE FROM projects WHERE id = ?1", [id])?;
    tx.execute("DELETE FROM boards WHERE id = ?1", [project.board_id])?;
    tx.commit()?;
    Ok(trashed)
}

pub fn rename_board(conn: &Connection, id: &str, name: &str) -> DbResult<Board> {
    let name = clean_name(name)?;
    require_personal_board(conn, id)?;
    conn.execute(
        "UPDATE boards SET name = ?2 WHERE id = ?1",
        params![id, name],
    )?;
    require_personal_board(conn, id)
}

pub fn set_board_archived(
    conn: &Connection,
    id: &str,
    archived: bool,
    now: DateTime<Utc>,
) -> DbResult<Board> {
    require_personal_board(conn, id)?;
    conn.execute(
        "UPDATE boards SET archived_at = ?2 WHERE id = ?1",
        params![id, archived.then_some(now)],
    )?;
    require_personal_board(conn, id)
}

/// Moves the board's own cards (those not in a project) to the Trash and
/// deletes the board. Project cards placed on it simply lose that placement.
pub fn delete_board(conn: &Connection, id: &str, now: DateTime<Utc>) -> DbResult<usize> {
    require_personal_board(conn, id)?;
    let tx = conn.unchecked_transaction()?;
    let trashed = tx.execute(
        "UPDATE items SET deleted_at = ?2 WHERE deleted_at IS NULL AND project_id IS NULL \
         AND id IN (SELECT item_id FROM placements WHERE board_id = ?1)",
        params![id, now],
    )?;
    tx.execute("DELETE FROM boards WHERE id = ?1", [id])?;
    tx.commit()?;
    Ok(trashed)
}

pub fn list_archived_projects(conn: &Connection) -> DbResult<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM projects WHERE archived_at IS NOT NULL ORDER BY archived_at DESC",
    )?;
    let rows = stmt.query_map([], row_to_project)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn list_archived_boards(conn: &Connection) -> DbResult<Vec<Board>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM boards WHERE archived_at IS NOT NULL AND kind = 'personal' \
         ORDER BY archived_at DESC",
    )?;
    let rows = stmt.query_map([], row_to_board)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{
        self,
        boards::create_board,
        cards::list_trashed,
        items::create_item,
        models::{ItemType, NewItem},
        projects::{create_project, list_projects},
    };

    #[test]
    fn rename_recolor_archive_project() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let p = create_project(&conn, "Old", "#79B8F5", now).unwrap();
        let p = rename_project(&conn, &p.id, "New").unwrap();
        assert_eq!(p.name, "New");
        assert!(set_project_color(&conn, &p.id, "red").is_err());
        set_project_color(&conn, &p.id, "#AABBCC").unwrap();
        set_project_archived(&conn, &p.id, true, now).unwrap();
        assert!(list_projects(&conn).unwrap().is_empty());
        assert_eq!(list_archived_projects(&conn).unwrap().len(), 1);
    }

    #[test]
    fn deleting_places_trashes_their_cards() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let p = create_project(&conn, "P", "#79B8F5", now).unwrap();
        let b = create_board(&conn, "B", now).unwrap();
        for (project_id, board_id) in [(Some(p.id.clone()), None), (None, Some(b.id.clone()))] {
            create_item(
                &conn,
                NewItem {
                    kind: ItemType::Task,
                    title: "card".into(),
                    project_id,
                    board_id,
                    ..Default::default()
                },
                now,
            )
            .unwrap();
        }
        assert_eq!(delete_project(&conn, &p.id, now).unwrap(), 1);
        assert_eq!(delete_board(&conn, &b.id, now).unwrap(), 1);
        assert_eq!(list_trashed(&conn).unwrap().len(), 2);
    }
}
