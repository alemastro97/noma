//! "Break down": the model proposes 3–7 next steps for a task; they become
//! task cards in the same place, each linked to the parent.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::ai::AiError;
use crate::db::{
    self, DbResult,
    models::{BoardKind, Item, ItemType, NewItem, Status},
};

pub const MIN_STEPS: usize = 3;
pub const MAX_STEPS: usize = 7;
/// Label on the link from a step to its parent.
pub const STEP_LABEL: &str = "step";

pub const SYSTEM_PROMPT: &str = "You break one task from the user's personal organizer into concrete next steps. \
Give 3 to 7 steps in the order they should happen. Each step is a short imperative headline of 2 to 8 words, \
in the same language as the task (Italian or English), first letter capitalized. \
Only steps the user can actually do; no dates, no numbering, never repeat the task itself. \
Reply with compact JSON that matches the schema.";

pub fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "steps": {
                "type": "array",
                "minItems": MIN_STEPS,
                "maxItems": MAX_STEPS,
                "items": { "type": "string", "minLength": 2, "maxLength": 80 }
            }
        },
        "required": ["steps"]
    })
}

pub fn user_prompt(item: &Item, project: Option<&str>) -> String {
    let mut out = format!("Task: {}", item.title);
    if let Some(body) = item.body.as_deref().filter(|b| !b.trim().is_empty()) {
        let body: String = body.chars().take(600).collect();
        out.push_str(&format!("\nNotes: {body}"));
    }
    if let Some(p) = project {
        out.push_str(&format!("\nProject: {p}"));
    }
    out
}

#[derive(Deserialize)]
struct Reply {
    steps: Vec<String>,
}

/// "1. Call" / "2) Call" → "Call"; "3D print" stays.
fn strip_number(s: &str) -> &str {
    let rest = s.trim_start_matches(|c: char| c.is_ascii_digit());
    if rest.len() < s.len()
        && let Some(r) = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')'))
    {
        return r.trim_start();
    }
    s
}

/// Cleans the model's steps: trimmed, deduplicated, no copy of the parent title.
pub fn parse(reply: &str, parent_title: &str) -> Result<Vec<String>, AiError> {
    let r: Reply = super::parse_reply(reply)?;
    let mut steps: Vec<String> = Vec::new();
    for s in r.steps {
        let s: String = strip_number(s.trim())
            .trim_start_matches(['-', '•', ' '])
            .trim_end_matches('.')
            .chars()
            .take(80)
            .collect();
        let dup = steps.iter().any(|x| x.eq_ignore_ascii_case(&s));
        if !s.is_empty() && !dup && !s.eq_ignore_ascii_case(parent_title.trim()) {
            steps.push(s);
        }
    }
    steps.truncate(MAX_STEPS);
    if steps.len() < 2 {
        return Err(AiError::BadOutput("too few steps".into()));
    }
    Ok(steps)
}

/// Creates the steps next to `parent` (same project, or same personal board)
/// and links each one to it. Returns the new ids in order.
pub fn create_steps(
    conn: &Connection,
    parent: &Item,
    steps: &[String],
    now: DateTime<Utc>,
) -> DbResult<Vec<String>> {
    let board: Option<String> = if parent.project_id.is_some() {
        None
    } else {
        conn.query_row(
            "SELECT pl.board_id FROM placements pl JOIN boards b ON b.id = pl.board_id \
             WHERE pl.item_id = ?1 AND b.kind = ?2",
            rusqlite::params![parent.id, BoardKind::Personal],
            |r| r.get(0),
        )
        .optional()?
    };
    let status = match parent.status {
        Status::Done => Status::Inbox,
        s => s,
    };
    let mut ids = Vec::new();
    for title in steps {
        let item = db::items::create_item(
            conn,
            NewItem {
                kind: ItemType::Task,
                title: title.clone(),
                status: Some(status),
                project_id: parent.project_id.clone(),
                board_id: board.clone(),
                ..Default::default()
            },
            now,
        )?;
        db::links::create_link(conn, &item.id, &parent.id, Some(STEP_LABEL), now)?;
        ids.push(item.id);
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_are_cleaned_and_land_with_the_parent() {
        let steps = parse(
            r#"{"steps":["1. Call the venue","Call the venue","Plan the trip","Send the deposit."]}"#,
            "Plan the trip",
        )
        .unwrap();
        assert_eq!(steps, vec!["Call the venue", "Send the deposit"]);

        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let project = db::projects::create_project(&conn, "Launch", "#fff", now).unwrap();
        let parent = db::items::create_item(
            &conn,
            NewItem {
                title: "Plan the offsite".into(),
                project_id: Some(project.id.clone()),
                ..Default::default()
            },
            now,
        )
        .unwrap();
        let ids = create_steps(&conn, &parent, &steps, now).unwrap();
        assert_eq!(ids.len(), 2);
        let first = db::items::get_item(&conn, &ids[0]).unwrap().unwrap();
        assert_eq!(first.project_id.as_deref(), Some(project.id.as_str()));
        assert_eq!(first.link_count, 1);
    }
}
