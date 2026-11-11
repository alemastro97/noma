use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row, params};

use super::boards::insert_board;
use super::error::{DbError, DbResult};
use super::models::{BoardKind, Project, new_id};

pub(crate) fn row_to_project(row: &Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get("id")?,
        name: row.get("name")?,
        color: row.get("color")?,
        board_id: row.get("board_id")?,
        created_at: row.get("created_at")?,
    })
}

/// Creates a project and the board it owns.
pub fn create_project(
    conn: &Connection,
    name: &str,
    color: &str,
    now: DateTime<Utc>,
) -> DbResult<Project> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DbError::Invalid("project name must not be blank".into()));
    }
    let tx = conn.unchecked_transaction()?;
    let board = insert_board(&tx, name, BoardKind::Project, now)?;
    let project = Project {
        id: new_id(),
        name: name.to_string(),
        color: color.to_string(),
        board_id: board.id,
        created_at: now,
    };
    tx.execute(
        "INSERT INTO projects (id, name, color, board_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            project.id,
            project.name,
            project.color,
            project.board_id,
            project.created_at
        ],
    )?;
    tx.commit()?;
    Ok(project)
}

pub fn list_projects(conn: &Connection) -> DbResult<Vec<Project>> {
    let mut stmt =
        conn.prepare("SELECT * FROM projects WHERE archived_at IS NULL ORDER BY created_at ASC")?;
    let rows = stmt.query_map([], row_to_project)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn get_project(conn: &Connection, id: &str) -> DbResult<Option<Project>> {
    let mut stmt = conn.prepare("SELECT * FROM projects WHERE id = ?1")?;
    let mut rows = stmt.query_map([id], row_to_project)?;
    rows.next().transpose().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn create_project_creates_board() {
        let conn = db::open_in_memory().unwrap();
        let p = create_project(&conn, "Spring launch", "#79B8F5", Utc::now()).unwrap();
        let board = db::boards::get_board(&conn, &p.board_id).unwrap().unwrap();
        assert_eq!(board.kind, BoardKind::Project);
        assert_eq!(board.name, "Spring launch");
        assert_eq!(list_projects(&conn).unwrap().len(), 1);
    }
}
