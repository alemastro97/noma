//! Project briefs: a short AI-written status paragraph and the suggested next
//! action, stored per project (`project_briefs`) and regenerated on request.

use chrono::{DateTime, Duration, Utc};
use chrono_tz::Tz;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::ai::AiError;
use crate::db::{
    DbResult,
    items::list_items,
    models::{Item, ItemFilter, ItemType, Status},
};

/// Cards per section sent to the model.
const MAX_DONE: usize = 12;
const MAX_OPEN: usize = 20;

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectBrief {
    pub project_id: String,
    pub summary: Option<String>,
    pub summary_at: Option<DateTime<Utc>>,
    pub next_action: Option<String>,
    pub next_item_id: Option<String>,
    pub next_at: Option<DateTime<Utc>>,
}

/// The stored brief. A next action whose card was finished or deleted is
/// stale and left out.
pub fn get(conn: &Connection, project_id: &str) -> DbResult<ProjectBrief> {
    let brief = conn
        .query_row(
            "SELECT b.summary, b.summary_at, b.next_action, b.next_item_id, b.next_at, \
               (SELECT i.status FROM items i WHERE i.id = b.next_item_id) \
             FROM project_briefs b WHERE b.project_id = ?1",
            [project_id],
            |r| {
                let open = r.get::<_, Option<String>>(5)?.is_some_and(|s| s != "done");
                Ok(ProjectBrief {
                    project_id: project_id.to_string(),
                    summary: r.get(0)?,
                    summary_at: r.get(1)?,
                    next_action: if open { r.get(2)? } else { None },
                    next_item_id: if open { r.get(3)? } else { None },
                    next_at: if open { r.get(4)? } else { None },
                })
            },
        )
        .optional()?;
    Ok(brief.unwrap_or_else(|| ProjectBrief {
        project_id: project_id.to_string(),
        ..Default::default()
    }))
}

pub fn set_summary(
    conn: &Connection,
    project_id: &str,
    summary: &str,
    now: DateTime<Utc>,
) -> DbResult<()> {
    conn.execute(
        "INSERT INTO project_briefs (project_id, summary, summary_at) VALUES (?1, ?2, ?3) \
         ON CONFLICT(project_id) DO UPDATE SET summary = excluded.summary, summary_at = excluded.summary_at",
        params![project_id, summary, now],
    )?;
    Ok(())
}

pub fn set_next(
    conn: &Connection,
    project_id: &str,
    action: Option<&str>,
    item_id: Option<&str>,
    now: DateTime<Utc>,
) -> DbResult<()> {
    conn.execute(
        "INSERT INTO project_briefs (project_id, next_action, next_item_id, next_at) VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT(project_id) DO UPDATE SET next_action = excluded.next_action, \
         next_item_id = excluded.next_item_id, next_at = excluded.next_at",
        params![project_id, action, item_id, now],
    )?;
    Ok(())
}

/// A project's cards: done in the last 30 days (newest first) and open ones
/// (soonest first, as the list view orders them).
pub fn cards(
    conn: &Connection,
    project_id: &str,
    now: DateTime<Utc>,
) -> DbResult<(Vec<Item>, Vec<Item>)> {
    let all = list_items(conn, &ItemFilter::Project(project_id.to_string()), true)?;
    let all: Vec<Item> = all
        .into_iter()
        .filter(|i| i.kind != ItemType::ProjectRef)
        .collect();
    let mut done: Vec<Item> = all
        .iter()
        .filter(|i| i.status == Status::Done)
        .filter(|i| {
            i.completed_at
                .is_some_and(|c| c >= now - Duration::days(30))
        })
        .cloned()
        .collect();
    done.sort_by(|a, b| b.completed_at.cmp(&a.completed_at));
    done.truncate(MAX_DONE);
    let mut open: Vec<Item> = all
        .into_iter()
        .filter(|i| i.status != Status::Done)
        .collect();
    open.truncate(MAX_OPEN);
    Ok((done, open))
}

pub const SUMMARY_PROMPT: &str = "You write a short status update for one project in the user's personal organizer. \
In 2 to 4 plain sentences (at most 70 words): what was done recently, what is still open, and what should come next. \
Write in the language of the cards (Italian or English), addressed to the user as \"you\". \
Only use the cards given; never invent facts, dates or people. No lists, no markdown. \
Reply with compact JSON that matches the schema.";

pub fn summary_schema() -> Value {
    json!({
        "type": "object",
        "properties": { "summary": { "type": "string", "minLength": 1, "maxLength": 600 } },
        "required": ["summary"]
    })
}

fn bullet_list(items: &[Item], tz: Tz) -> String {
    if items.is_empty() {
        return "(none)".into();
    }
    items
        .iter()
        .map(|i| format!("- {}", super::card_line(i, tz)))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn summary_prompt(
    name: &str,
    done: &[Item],
    open: &[Item],
    now: DateTime<Utc>,
    tz: Tz,
) -> String {
    format!(
        "Today: {}\nProject: {name}\nDone recently:\n{}\nOpen:\n{}",
        now.with_timezone(&tz).format("%Y-%m-%d"),
        bullet_list(done, tz),
        bullet_list(open, tz)
    )
}

#[derive(Deserialize)]
struct SummaryReply {
    summary: String,
}

pub fn parse_summary(reply: &str) -> Result<String, AiError> {
    let r: SummaryReply = super::parse_reply(reply)?;
    let s = r.summary.trim().to_string();
    if s.is_empty() {
        return Err(AiError::BadOutput("empty summary".into()));
    }
    Ok(s)
}

/// Open cards that can be a next action (not notes or ideas).
pub fn actionable(open: &[Item]) -> Vec<Item> {
    open.iter()
        .filter(|i| {
            matches!(
                i.kind,
                ItemType::Task | ItemType::Reminder | ItemType::Event
            )
        })
        .cloned()
        .collect()
}

pub const NEXT_PROMPT: &str = "You pick the single next action for one project in the user's personal organizer. \
From the numbered open cards, choose the one the user should do next: urgent or overdue first, then what unblocks the rest, \
then what is in progress. Give its number in \"pick\" and in \"action\" a short imperative phrasing of that card \
(3 to 10 words, the card's language). Reply with compact JSON that matches the schema.";

pub fn next_schema(n: usize) -> Value {
    json!({
        "type": "object",
        "properties": {
            "pick": { "type": "integer", "minimum": 1, "maximum": n.max(1) },
            "action": { "type": "string", "minLength": 1, "maxLength": 100 }
        },
        "required": ["pick", "action"]
    })
}

pub fn next_prompt(name: &str, cards: &[Item], now: DateTime<Utc>, tz: Tz) -> String {
    let list = cards
        .iter()
        .enumerate()
        .map(|(n, i)| format!("{}. {}", n + 1, super::card_line(i, tz)))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Today: {}\nProject: {name}\nOpen cards:\n{list}",
        now.with_timezone(&tz).format("%Y-%m-%d")
    )
}

#[derive(Deserialize)]
struct NextReply {
    pick: usize,
    action: String,
}

/// (card index, action text).
pub fn parse_next(reply: &str, n: usize) -> Result<(usize, String), AiError> {
    let r: NextReply = super::parse_reply(reply)?;
    if r.pick == 0 || r.pick > n {
        return Err(AiError::BadOutput(format!("pick {} out of range", r.pick)));
    }
    Ok((
        r.pick - 1,
        r.action.trim().trim_end_matches('.').to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{self, models::NewItem};

    #[test]
    fn brief_roundtrips_and_a_finished_next_action_goes_stale() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let p = db::projects::create_project(&conn, "Launch", "#fff", now).unwrap();
        assert_eq!(get(&conn, &p.id).unwrap().summary, None);
        let card = db::items::create_item(
            &conn,
            NewItem {
                title: "Book venue".into(),
                project_id: Some(p.id.clone()),
                ..Default::default()
            },
            now,
        )
        .unwrap();
        set_summary(&conn, &p.id, "On track.", now).unwrap();
        set_next(&conn, &p.id, Some("Book the venue"), Some(&card.id), now).unwrap();
        let b = get(&conn, &p.id).unwrap();
        assert_eq!(b.summary.as_deref(), Some("On track."));
        assert_eq!(b.next_action.as_deref(), Some("Book the venue"));
        db::items::complete_item(&conn, &card.id, now, chrono_tz::UTC).unwrap();
        let b = get(&conn, &p.id).unwrap();
        assert_eq!(b.next_action, None);
        assert_eq!(b.summary.as_deref(), Some("On track."));
    }

    #[test]
    fn next_pick_must_be_in_range() {
        assert_eq!(
            parse_next(r#"{"pick":2,"action":"Call Luca."}"#, 3).unwrap(),
            (1, "Call Luca".to_string())
        );
        assert!(parse_next(r#"{"pick":4,"action":"x"}"#, 3).is_err());
    }
}
