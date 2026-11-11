//! The weekly review: what got done in the last 7 days, what is overdue and
//! what has sat untouched for 30+ days ("forgotten"), plus a short model-written
//! summary of the week.

use chrono::{DateTime, Duration, Utc};
use chrono_tz::Tz;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::ai::AiError;
use crate::db::{self, DbResult, models::Item};

pub const WEEK_DAYS: i64 = 7;
pub const FORGOTTEN_DAYS: i64 = 30;
const MAX_PER_SECTION: u32 = 50;

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub completed: Vec<Item>,
    pub overdue: Vec<Item>,
    pub forgotten: Vec<Item>,
}

fn ids(conn: &Connection, sql: &str, at: DateTime<Utc>) -> DbResult<Vec<Item>> {
    let mut stmt = conn.prepare(sql)?;
    let ids: Vec<String> = stmt
        .query_map(rusqlite::params![at, MAX_PER_SECTION], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut out = Vec::new();
    for id in ids {
        if let Some(item) = db::items::get_item(conn, &id)? {
            out.push(item);
        }
    }
    Ok(out)
}

pub fn build(conn: &Connection, now: DateTime<Utc>) -> DbResult<Review> {
    let mut r = Review {
        completed: ids(
            conn,
            "SELECT id FROM items WHERE status = 'done' AND kind != 'project_ref' \
             AND completed_at >= ?1 ORDER BY completed_at DESC LIMIT ?2",
            now - Duration::days(WEEK_DAYS),
        )?,
        // Events are over, not overdue.
        overdue: ids(
            conn,
            "SELECT id FROM items WHERE status != 'done' AND kind IN ('task', 'reminder') \
             AND COALESCE(due_at, remind_at) < ?1 ORDER BY COALESCE(due_at, remind_at) ASC LIMIT ?2",
            now,
        )?,
        forgotten: ids(
            conn,
            "SELECT id FROM items WHERE status != 'done' AND kind != 'project_ref' \
             AND updated_at < ?1 ORDER BY updated_at ASC LIMIT ?2",
            now - Duration::days(FORGOTTEN_DAYS),
        )?,
    };
    // Overdue is the louder signal: a card is listed once.
    let overdue: Vec<String> = r.overdue.iter().map(|i| i.id.clone()).collect();
    r.forgotten.retain(|i| !overdue.contains(&i.id));
    Ok(r)
}

pub const SUMMARY_PROMPT: &str = "You write the user's weekly review for their personal organizer. \
Exactly 3 short sentences, addressed to the user as \"you\": what they got done, what is overdue or slipping, \
and one gentle suggestion for the week ahead. Warm and calm, never guilt-tripping. \
Write in the language of the cards (Italian or English). Only use the cards given; never invent anything. \
No lists, no markdown. Reply with compact JSON that matches the schema.";

pub fn schema() -> Value {
    json!({
        "type": "object",
        "properties": { "summary": { "type": "string", "minLength": 1, "maxLength": 500 } },
        "required": ["summary"]
    })
}

fn section(title: &str, items: &[Item], tz: Tz) -> String {
    let lines: Vec<String> = items
        .iter()
        .take(15)
        .map(|i| format!("- {}", super::card_line(i, tz)))
        .collect();
    let more = items.len().saturating_sub(15);
    let mut out = format!("{title} ({}):\n", items.len());
    out.push_str(if lines.is_empty() { "(none)" } else { "" });
    out.push_str(&lines.join("\n"));
    if more > 0 {
        out.push_str(&format!("\n- and {more} more"));
    }
    out
}

pub fn user_prompt(r: &Review, now: DateTime<Utc>, tz: Tz) -> String {
    format!(
        "Today: {}\n{}\n{}\n{}",
        now.with_timezone(&tz).format("%Y-%m-%d (%A)"),
        section("Completed in the last 7 days", &r.completed, tz),
        section("Overdue", &r.overdue, tz),
        section("Untouched for 30+ days", &r.forgotten, tz)
    )
}

#[derive(Deserialize)]
struct Reply {
    summary: String,
}

pub fn parse(reply: &str) -> Result<String, AiError> {
    let r: Reply = super::parse_reply(reply)?;
    let s = r.summary.trim().to_string();
    if s.is_empty() {
        return Err(AiError::BadOutput("empty summary".into()));
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{ItemType, NewItem};

    #[test]
    fn sections_pick_done_overdue_and_forgotten() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let old = now - Duration::days(40);
        let mk = |title: &str, kind: ItemType, due: Option<DateTime<Utc>>, at: DateTime<Utc>| {
            db::items::create_item(
                &conn,
                NewItem {
                    kind,
                    title: title.into(),
                    due_at: due,
                    ..Default::default()
                },
                at,
            )
            .unwrap()
        };
        let done = mk("Shipped it", ItemType::Task, None, now);
        db::items::complete_item(&conn, &done.id, now, chrono_tz::UTC).unwrap();
        let late = mk(
            "Pay rent",
            ItemType::Task,
            Some(now - Duration::days(2)),
            now,
        );
        mk(
            "Old event",
            ItemType::Event,
            Some(now - Duration::days(2)),
            now,
        );
        let dusty = mk("Learn sourdough", ItemType::Idea, None, old);
        mk("Fresh idea", ItemType::Idea, None, now);

        let r = build(&conn, now).unwrap();
        assert_eq!(
            r.completed.iter().map(|i| &i.id).collect::<Vec<_>>(),
            vec![&done.id]
        );
        assert_eq!(
            r.overdue.iter().map(|i| &i.id).collect::<Vec<_>>(),
            vec![&late.id]
        );
        assert_eq!(
            r.forgotten.iter().map(|i| &i.id).collect::<Vec<_>>(),
            vec![&dusty.id]
        );
        assert!(user_prompt(&r, now, chrono_tz::UTC).contains("Overdue (1):\n- Pay rent"));
    }
}
