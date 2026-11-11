//! Re-sort Inbox: re-runs filing (project / board) for open Inbox cards that
//! came from a capture, with today's projects, boards and the user's filing
//! corrections. Only the place changes; the card is otherwise left alone.

use rusqlite::Connection;

use crate::ai::extract::Extracted;
use crate::db::{
    self, DbResult,
    filing::Place,
    models::{BoardKind, ItemFilter},
};

/// Cards re-filed per run: each one is a model call.
pub const MAX_CARDS: usize = 25;

/// An Inbox card and the text the user originally captured.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub item_id: String,
    pub text: String,
}

/// Open Inbox cards (oldest first) whose capture finished, with its text.
pub fn candidates(conn: &Connection) -> DbResult<Vec<Candidate>> {
    let mut items = db::items::list_items(conn, &ItemFilter::Inbox, false)?;
    items.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    let mut out = Vec::new();
    for item in items.into_iter().filter(|i| !i.capturing) {
        if let Some(c) = db::captures::latest_for_item(conn, &item.id)?
            && c.status == "done"
        {
            out.push(Candidate {
                item_id: item.id,
                text: c.raw_text,
            });
        }
        if out.len() == MAX_CARDS {
            break;
        }
    }
    Ok(out)
}

/// Owned filing context, the same the capture worker gives the model.
pub struct FilingContext {
    pub projects: Vec<(String, String)>,
    pub boards: Vec<(String, String)>,
    pub recent: Vec<(String, Vec<String>)>,
    pub examples: Vec<(String, String)>,
}

pub fn filing_context(conn: &Connection) -> DbResult<FilingContext> {
    let projects: Vec<(String, String)> = db::projects::list_projects(conn)?
        .into_iter()
        .map(|p| (p.id, p.name))
        .collect();
    let boards: Vec<(String, String)> = db::boards::list_boards(conn)?
        .into_iter()
        .filter(|b| b.kind == BoardKind::Personal)
        .map(|b| (b.id, b.name))
        .collect();
    let mut recent = Vec::new();
    for (id, _) in &projects {
        recent.push((id.clone(), db::items::recent_titles(conn, id, false, 3)?));
    }
    for (id, _) in &boards {
        recent.push((id.clone(), db::items::recent_titles(conn, id, true, 3)?));
    }
    Ok(FilingContext {
        projects,
        boards,
        recent,
        examples: db::filing::examples(conn, 60)?,
    })
}

/// Where the model's first card says this note belongs; `None` keeps it in the Inbox.
pub fn place_for(extracted: &[Extracted]) -> Option<Place> {
    let first = extracted.first()?;
    if first.creates_project {
        return None;
    }
    first
        .project_id
        .clone()
        .map(Place::Project)
        .or_else(|| first.board_id.clone().map(Place::Board))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn only_finished_captures_in_the_inbox_are_candidates() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let mk = |title: &str| {
            db::items::create_item(
                &conn,
                db::models::NewItem {
                    title: title.into(),
                    ..Default::default()
                },
                now,
            )
            .unwrap()
            .id
        };
        let typed = mk("typed by hand");
        let captured = mk("Call the movers");
        let cap =
            db::captures::create_capture(&conn, "text", "call the movers", Some(&captured), now)
                .unwrap();
        assert!(candidates(&conn).unwrap().is_empty());
        db::captures::set_status(&conn, &cap, db::captures::CaptureStatus::Done, None, None)
            .unwrap();
        let c = candidates(&conn).unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].item_id, captured);
        assert_ne!(c[0].item_id, typed);
    }
}
