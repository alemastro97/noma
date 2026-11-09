//! Writing extraction results: the capture's placeholder card becomes the first
//! extracted card in place (same id, same canvas spot); any further cards are
//! created next to it.

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use rusqlite::{Connection, OptionalExtension};

use super::extract::Extracted;
use crate::db::{
    self, DbResult,
    models::{ItemPatch, NewItem},
};

/// Same palette and order as `src/lib/colors.ts`.
const PROJECT_COLORS: [&str; 6] = [
    "#79B8F5", "#78DCC0", "#F2BD73", "#DDA1C5", "#A78BFA", "#F28C91",
];

fn patch_from(e: &Extracted, project_id: Option<String>) -> ItemPatch {
    ItemPatch {
        kind: Some(e.kind),
        title: Some(e.title.clone()),
        body: Some(e.body.clone()),
        priority: Some(e.priority),
        due_at: Some(e.due_at),
        ends_at: Some(e.ends_at),
        remind_at: Some(e.remind_at),
        recurrence: Some(e.recurrence.clone()),
        project_id: project_id.map(Some),
        tags: Some(e.tags.clone()),
        hint: Some(e.hint.clone()),
        ..Default::default()
    }
}

fn create_project_for(conn: &Connection, name: &str, now: DateTime<Utc>) -> DbResult<String> {
    let n = db::projects::list_projects(conn)?.len();
    let color = PROJECT_COLORS[n % PROJECT_COLORS.len()];
    Ok(db::projects::create_project(conn, name, color, now)?.id)
}

/// Applies `extracted` for the capture whose placeholder is `placeholder_id`
/// (None when the user deleted it meanwhile). Returns the ids of every card
/// written, the placeholder first.
pub fn apply(
    conn: &Connection,
    placeholder_id: Option<&str>,
    extracted: &[Extracted],
    now: DateTime<Utc>,
    tz: Tz,
) -> DbResult<Vec<String>> {
    let placeholder = match placeholder_id {
        Some(id) => db::items::get_item(conn, id)?,
        None => None,
    };
    // Where the placeholder lives: extra cards go to the same board.
    let board: Option<String> = match &placeholder {
        Some(p) => conn
            .query_row(
                "SELECT board_id FROM placements WHERE item_id = ?1",
                [&p.id],
                |r| r.get(0),
            )
            .optional()?,
        None => None,
    };
    // Context chosen by the user (project view, board) wins over the model's guess.
    let context_project = placeholder.as_ref().and_then(|p| p.project_id.clone());

    let mut ids = Vec::new();
    for (index, e) in extracted.iter().enumerate() {
        let project_id = if e.creates_project {
            Some(create_project_for(conn, &e.title, now)?)
        } else {
            context_project.clone().or_else(|| e.project_id.clone())
        };

        match (&placeholder, index) {
            (Some(p), 0) => {
                let mut patch = patch_from(e, None);
                if p.project_id != project_id && project_id.is_some() {
                    patch.project_id = Some(project_id.clone());
                }
                db::items::update_item(conn, &p.id, patch, now, tz)?;
                ids.push(p.id.clone());
            }
            _ => {
                let item = db::items::create_item(
                    conn,
                    NewItem {
                        kind: e.kind,
                        title: e.title.clone(),
                        body: e.body.clone(),
                        priority: Some(e.priority),
                        status: placeholder.as_ref().map(|p| p.status),
                        due_at: e.due_at,
                        ends_at: e.ends_at,
                        remind_at: e.remind_at,
                        recurrence: e.recurrence.clone(),
                        board_id: if project_id.is_some() {
                            None
                        } else {
                            board.clone()
                        },
                        project_id,
                        tags: Some(e.tags.clone()),
                        x: None,
                        y: None,
                        hint: e.hint.clone(),
                    },
                    now,
                )?;
                ids.push(item.id);
            }
        }
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{CaptureHint, ItemType, Priority};
    use chrono::TimeZone;

    fn ex(kind: ItemType, title: &str) -> Extracted {
        Extracted {
            kind,
            creates_project: false,
            title: title.into(),
            body: None,
            due_at: None,
            ends_at: None,
            remind_at: None,
            recurrence: None,
            priority: Priority::Normal,
            project_id: None,
            tags: vec![],
            hint: None,
        }
    }

    #[test]
    fn placeholder_is_rewritten_in_place_and_extras_join_its_board() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 9, 9, 0, 0).unwrap();
        let board = db::boards::create_board(&conn, "B", now).unwrap();
        let placeholder = db::items::create_item(
            &conn,
            NewItem {
                kind: ItemType::Note,
                title: "buy milk and call mum tomorrow".into(),
                board_id: Some(board.id.clone()),
                x: Some(400.0),
                y: Some(300.0),
                ..Default::default()
            },
            now,
        )
        .unwrap();
        let mut call = ex(ItemType::Reminder, "Call mum");
        call.remind_at = Some(Utc.with_ymd_and_hms(2026, 11, 10, 8, 0, 0).unwrap());
        call.hint = Some(CaptureHint {
            low_confidence: true,
            ..Default::default()
        });
        let ids = apply(
            &conn,
            Some(&placeholder.id),
            &[ex(ItemType::Task, "Buy milk"), call],
            now,
            chrono_tz::UTC,
        )
        .unwrap();
        assert_eq!(ids[0], placeholder.id);
        let first = db::items::get_item(&conn, &ids[0]).unwrap().unwrap();
        assert_eq!(first.kind, ItemType::Task);
        assert_eq!(first.title, "Buy milk");
        let second = db::items::get_item(&conn, &ids[1]).unwrap().unwrap();
        assert_eq!(second.kind, ItemType::Reminder);
        assert!(second.hint.unwrap().low_confidence);
        assert_eq!(
            db::placements::list_placements(&conn, &board.id)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn project_capture_creates_project_and_moves_card_to_its_board() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let placeholder = db::items::create_item(
            &conn,
            NewItem {
                kind: ItemType::Note,
                title: "new project kitchen renovation".into(),
                ..Default::default()
            },
            now,
        )
        .unwrap();
        let mut e = ex(ItemType::ProjectRef, "Kitchen renovation");
        e.creates_project = true;
        let ids = apply(&conn, Some(&placeholder.id), &[e], now, chrono_tz::UTC).unwrap();
        let card = db::items::get_item(&conn, &ids[0]).unwrap().unwrap();
        let projects = db::projects::list_projects(&conn).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(card.project_id.as_deref(), Some(projects[0].id.as_str()));
        assert_eq!(
            db::placements::list_placements(&conn, &projects[0].board_id)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn deleted_placeholder_still_produces_cards() {
        let conn = db::open_in_memory().unwrap();
        let ids = apply(
            &conn,
            None,
            &[ex(ItemType::Idea, "Dim the orb")],
            Utc::now(),
            chrono_tz::UTC,
        )
        .unwrap();
        assert_eq!(ids.len(), 1);
    }
}
