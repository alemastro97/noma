//! Where a card lives (a project, a personal board or the Inbox), moving it
//! there, and remembering the user's corrections so the model files similar
//! notes the same way next time.

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use super::error::DbResult;
use super::models::{ItemPatch, new_id};
use super::{items, placements};

pub const INBOX_NAME: &str = "Inbox";
/// Corrections kept; older ones are dropped.
const MAX_EXAMPLES: u32 = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum Place {
    Project(String),
    Board(String),
    Inbox,
}

impl Place {
    fn parts(&self) -> (&'static str, Option<&str>) {
        match self {
            Self::Project(id) => ("project", Some(id)),
            Self::Board(id) => ("board", Some(id)),
            Self::Inbox => ("inbox", None),
        }
    }
}

/// The place a card is in and its display name. A card on a project's board
/// counts as the project; no project and no board is the Inbox.
pub fn place_of(conn: &Connection, item_id: &str) -> DbResult<(Place, String)> {
    let project: Option<(String, String)> = conn
        .query_row(
            "SELECT p.id, p.name FROM items i JOIN projects p ON p.id = i.project_id WHERE i.id = ?1",
            [item_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((id, name)) = project {
        return Ok((Place::Project(id), name));
    }
    let board: Option<(String, String)> = conn
        .query_row(
            "SELECT b.id, b.name FROM placements pl JOIN boards b ON b.id = pl.board_id \
             WHERE pl.item_id = ?1 AND b.kind = 'personal'",
            [item_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    Ok(match board {
        Some((id, name)) => (Place::Board(id), name),
        None => (Place::Inbox, INBOX_NAME.to_string()),
    })
}

/// Moves a card to `place`, leaving everything else about it alone.
pub fn move_item(
    conn: &Connection,
    item_id: &str,
    place: &Place,
    now: DateTime<Utc>,
    tz: Tz,
) -> DbResult<()> {
    let (current, _) = place_of(conn, item_id)?;
    if &current == place {
        return Ok(());
    }
    let set_project = |project: Option<String>| -> DbResult<()> {
        let patch = ItemPatch {
            project_id: Some(project),
            ..Default::default()
        };
        items::update_item(conn, item_id, patch, now, tz).map(|_| ())
    };
    match place {
        Place::Project(id) => {
            // Off a personal board first, so the project change places it on the project's.
            placements::remove_placement(conn, item_id)?;
            set_project(Some(id.clone()))
        }
        Place::Board(id) => {
            if matches!(current, Place::Project(_)) {
                set_project(None)?;
            }
            let (x, y) = items::next_slot(conn, id)?;
            placements::place_item(conn, item_id, id, x, y).map(|_| ())
        }
        Place::Inbox => {
            if matches!(current, Place::Project(_)) {
                set_project(None)?;
            }
            placements::remove_placement(conn, item_id)
        }
    }
}

/// Remembers that a note like `text` belongs in `place`. One example per text.
pub fn record(conn: &Connection, text: &str, place: &Place, now: DateTime<Utc>) -> DbResult<()> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(());
    }
    let (kind, id) = place.parts();
    conn.execute(
        "INSERT INTO filing_examples (id, text, place_kind, place_id, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5) \
         ON CONFLICT(text) DO UPDATE SET place_kind = excluded.place_kind, \
         place_id = excluded.place_id, created_at = excluded.created_at",
        params![new_id(), text, kind, id, now],
    )?;
    conn.execute(
        "DELETE FROM filing_examples WHERE id NOT IN \
         (SELECT id FROM filing_examples ORDER BY created_at DESC LIMIT ?1)",
        [MAX_EXAMPLES],
    )?;
    Ok(())
}

/// Records a correction for the capture behind `item_id`, if it has one.
pub fn record_for_item(
    conn: &Connection,
    item_id: &str,
    place: &Place,
    now: DateTime<Utc>,
) -> DbResult<()> {
    match super::captures::latest_for_item(conn, item_id)? {
        Some(c) if c.status == "done" => record(conn, &c.raw_text, place, now),
        _ => Ok(()),
    }
}

/// (note text, place name), newest first. Examples whose place was deleted are skipped.
pub fn examples(conn: &Connection, limit: u32) -> DbResult<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT e.text, CASE e.place_kind WHEN 'inbox' THEN ?2 \
           WHEN 'project' THEN (SELECT name FROM projects WHERE id = e.place_id) \
           ELSE (SELECT name FROM boards WHERE id = e.place_id) END AS place \
         FROM filing_examples e WHERE place IS NOT NULL \
         ORDER BY e.created_at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit, INBOX_NAME], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 4)
        .map(|w| w.chars().take(6).collect())
        .collect()
}

/// The `n` examples most like `text` (shared word stems), newest first among
/// equals. Examples sharing nothing with the note are left out: they would
/// only cost prompt tokens.
pub fn pick(text: &str, all: &[(String, String)], n: usize) -> Vec<(String, String)> {
    let mine = words(text);
    let mut scored: Vec<(usize, usize)> = all
        .iter()
        .enumerate()
        .map(|(i, (t, _))| {
            let theirs = words(t);
            (mine.iter().filter(|w| theirs.contains(w)).count(), i)
        })
        .filter(|(score, _)| *score > 0)
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored
        .into_iter()
        .take(n)
        .map(|(_, i)| all[i].clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{self, models::NewItem};

    fn item(conn: &Connection, title: &str) -> String {
        db::items::create_item(
            conn,
            NewItem {
                title: title.into(),
                ..Default::default()
            },
            Utc::now(),
        )
        .unwrap()
        .id
    }

    #[test]
    fn moves_between_inbox_project_and_board() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let tz = chrono_tz::UTC;
        let project = db::projects::create_project(&conn, "Studio move", "#fff", now).unwrap();
        let board = db::boards::create_board(&conn, "Trips", now).unwrap();
        let id = item(&conn, "call the movers");
        assert_eq!(place_of(&conn, &id).unwrap().0, Place::Inbox);

        move_item(&conn, &id, &Place::Project(project.id.clone()), now, tz).unwrap();
        assert_eq!(
            place_of(&conn, &id).unwrap(),
            (Place::Project(project.id.clone()), "Studio move".into())
        );
        assert_eq!(
            db::placements::list_placements(&conn, &project.board_id)
                .unwrap()
                .len(),
            1
        );

        move_item(&conn, &id, &Place::Board(board.id.clone()), now, tz).unwrap();
        assert_eq!(
            place_of(&conn, &id).unwrap().0,
            Place::Board(board.id.clone())
        );
        assert!(
            db::placements::list_placements(&conn, &project.board_id)
                .unwrap()
                .is_empty()
        );

        move_item(&conn, &id, &Place::Project(project.id.clone()), now, tz).unwrap();
        assert_eq!(place_of(&conn, &id).unwrap().0, Place::Project(project.id));
        move_item(&conn, &id, &Place::Inbox, now, tz).unwrap();
        assert_eq!(place_of(&conn, &id).unwrap().0, Place::Inbox);
        assert!(
            db::placements::list_placements(&conn, &board.id)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn corrections_are_remembered_once_and_picked_by_likeness() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let project = db::projects::create_project(&conn, "Studio move", "#fff", now).unwrap();
        record(&conn, "chiamare i traslocatori", &Place::Inbox, now).unwrap();
        record(
            &conn,
            "chiamare i traslocatori",
            &Place::Project(project.id.clone()),
            now,
        )
        .unwrap();
        record(&conn, "buy oat milk", &Place::Inbox, now).unwrap();
        let all = examples(&conn, 50).unwrap();
        assert_eq!(all.len(), 2);
        let picked = pick("sentire i traslocatori per il preventivo", &all, 3);
        assert_eq!(
            picked,
            vec![(
                "chiamare i traslocatori".to_string(),
                "Studio move".to_string()
            )]
        );
        assert!(pick("water the plants", &all, 3).is_empty());

        // A deleted place takes its examples with it.
        conn.execute("DELETE FROM projects WHERE id = ?1", [&project.id])
            .unwrap();
        assert_eq!(examples(&conn, 50).unwrap().len(), 1);
    }

    #[test]
    fn only_finished_captures_teach() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let id = item(&conn, "x");
        let cap = db::captures::create_capture(&conn, "text", "plan the offsite", Some(&id), now)
            .unwrap();
        record_for_item(&conn, &id, &Place::Inbox, now).unwrap();
        assert!(examples(&conn, 9).unwrap().is_empty());
        db::captures::set_status(&conn, &cap, db::captures::CaptureStatus::Done, None, None)
            .unwrap();
        record_for_item(&conn, &id, &Place::Inbox, now).unwrap();
        assert_eq!(examples(&conn, 9).unwrap().len(), 1);
    }
}
