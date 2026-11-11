//! Imports cards straight into the Inbox (or a project) without AI:
//! Markdown / plain text, a Todoist CSV export, Apple Notes and Reminders.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use super::apple;
use crate::commands::{emit_items_changed, lock_message, to_message};
use crate::db::{
    self, DbError, DbResult,
    models::{ItemType, NewItem, Priority, Status},
};
use crate::state::{AppState, local_tz, now};

const TITLE_MAX: usize = 120;
/// A runaway file must not flood the Inbox.
pub const MAX_CARDS: usize = 2000;

/// One card to create.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Draft {
    pub kind: ItemType,
    pub title: String,
    pub body: Option<String>,
    pub done: bool,
    pub priority: Option<Priority>,
    pub due_at: Option<DateTime<Utc>>,
    pub remind_at: Option<DateTime<Utc>>,
}

impl Draft {
    /// First line (trimmed to a title); the full text stays in the body when longer.
    pub fn from_text(kind: ItemType, text: &str) -> Option<Self> {
        let text = text.trim();
        let first = text.lines().next().unwrap_or("").trim();
        if first.is_empty() {
            return None;
        }
        let title: String = first.chars().take(TITLE_MAX).collect();
        let body = (text != title).then(|| text.to_string());
        Some(Self {
            kind,
            title,
            body,
            ..Default::default()
        })
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub created: usize,
    pub skipped: usize,
    pub ids: Vec<String>,
}

// ── Markdown / plain text ───────────────────────────────────────────────

fn list_item(line: &str) -> Option<&str> {
    let t = line.trim_start();
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = t.strip_prefix(marker) {
            return Some(rest);
        }
    }
    let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 {
        let rest = &t[digits..];
        if let Some(rest) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            return Some(rest);
        }
    }
    None
}

/// One card per list item (checkbox items become tasks) or paragraph (notes).
/// Headings and rules separate blocks and are not imported themselves.
pub fn parse_notes(text: &str) -> Vec<Draft> {
    let mut out: Vec<Draft> = Vec::new();
    let mut para: Vec<&str> = Vec::new();
    let mut in_item = false;
    let flush = |para: &mut Vec<&str>, out: &mut Vec<Draft>| {
        if let Some(d) = Draft::from_text(ItemType::Note, &para.join("\n")) {
            out.push(d);
        }
        para.clear();
    };
    for raw in text.trim_start_matches('\u{feff}').lines() {
        let line = raw.trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush(&mut para, &mut out);
            in_item = false;
            continue;
        }
        if trimmed.starts_with('#') || trimmed.chars().all(|c| c == '-' || c == '*' || c == '_') {
            flush(&mut para, &mut out);
            in_item = false;
            continue;
        }
        if let Some(rest) = list_item(line) {
            flush(&mut para, &mut out);
            let (done, rest) = if let Some(r) = rest.strip_prefix("[ ] ") {
                (Some(false), r)
            } else if let Some(r) = rest
                .strip_prefix("[x] ")
                .or_else(|| rest.strip_prefix("[X] "))
            {
                (Some(true), r)
            } else {
                (None, rest)
            };
            let kind = if done.is_some() {
                ItemType::Task
            } else {
                ItemType::Note
            };
            if let Some(mut d) = Draft::from_text(kind, rest) {
                d.done = done.unwrap_or(false);
                out.push(d);
                in_item = true;
            }
            continue;
        }
        // An indented line right after a list item continues it.
        if in_item && raw.starts_with([' ', '\t']) {
            if let Some(last) = out.last_mut() {
                let body = last.body.get_or_insert_with(|| last.title.clone());
                body.push('\n');
                body.push_str(trimmed);
            }
            continue;
        }
        in_item = false;
        para.push(trimmed);
    }
    flush(&mut para, &mut out);
    out
}

// ── Todoist CSV ─────────────────────────────────────────────────────────

/// RFC 4180-ish: quoted fields, doubled quotes, newlines inside quotes.
pub fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    field.push('"');
                    chars.next();
                }
                '"' => quoted = false,
                _ => field.push(c),
            }
            continue;
        }
        match c {
            '"' => quoted = true,
            ',' => row.push(std::mem::take(&mut field)),
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

/// Todoist's CSV export: one card per `task` row. Priority 1 is the highest.
/// Natural-language dates ("every monday") are kept in the body; plain
/// `YYYY-MM-DD` dates become the due date.
pub fn parse_todoist(text: &str) -> Result<Vec<Draft>, String> {
    let rows = parse_csv(text);
    let Some((header, rows)) = rows.split_first() else {
        return Ok(Vec::new());
    };
    let col = |name: &str| {
        header
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case(name))
    };
    let (Some(ty), Some(content)) = (col("TYPE"), col("CONTENT")) else {
        return Err("not a Todoist CSV export".into());
    };
    let (desc, prio, date) = (col("DESCRIPTION"), col("PRIORITY"), col("DATE"));
    let get = |row: &Vec<String>, i: Option<usize>| -> String {
        i.and_then(|i| row.get(i))
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };
    let tz = local_tz();
    let mut out = Vec::new();
    for row in rows {
        if !get(row, Some(ty)).eq_ignore_ascii_case("task") {
            continue;
        }
        let Some(mut d) = Draft::from_text(ItemType::Task, &get(row, Some(content))) else {
            continue;
        };
        let mut extra = Vec::new();
        let description = get(row, desc);
        if !description.is_empty() {
            extra.push(description);
        }
        let when = get(row, date);
        match NaiveDate::parse_from_str(&when, "%Y-%m-%d") {
            Ok(day) => {
                d.due_at = day
                    .and_hms_opt(9, 0, 0)
                    .and_then(|t| tz.from_local_datetime(&t).earliest())
                    .map(|t| t.with_timezone(&Utc));
            }
            Err(_) if !when.is_empty() => extra.push(format!("Todoist date: {when}")),
            Err(_) => {}
        }
        if !extra.is_empty() {
            let mut body = d.body.take().unwrap_or_else(|| d.title.clone());
            for e in extra {
                body.push_str("\n\n");
                body.push_str(&e);
            }
            d.body = Some(body);
        }
        d.priority = match get(row, prio).as_str() {
            "1" => Some(Priority::Urgent),
            "2" => Some(Priority::High),
            _ => None,
        };
        out.push(d);
    }
    Ok(out)
}

// ── Writing ─────────────────────────────────────────────────────────────

/// Creates the cards in the Inbox, or in `project_id`.
pub fn insert(
    conn: &Connection,
    drafts: Vec<Draft>,
    project_id: Option<&str>,
    now: DateTime<Utc>,
) -> DbResult<ImportSummary> {
    if let Some(p) = project_id
        && db::projects::get_project(conn, p)?.is_none()
    {
        return Err(DbError::NotFound(format!("project {p}")));
    }
    let skipped = drafts.len().saturating_sub(MAX_CARDS);
    let mut ids = Vec::new();
    for d in drafts.into_iter().take(MAX_CARDS) {
        let item = db::items::create_item(
            conn,
            NewItem {
                kind: d.kind,
                title: d.title,
                body: d.body,
                priority: d.priority,
                status: d.done.then_some(Status::Done),
                due_at: d.due_at,
                remind_at: d.remind_at,
                project_id: project_id.map(str::to_string),
                ..Default::default()
            },
            now,
        )?;
        ids.push(item.id);
    }
    Ok(ImportSummary {
        created: ids.len(),
        skipped,
        ids,
    })
}

fn save(
    app: &AppHandle,
    state: &AppState,
    drafts: Vec<Draft>,
    project_id: Option<String>,
) -> Result<ImportSummary, String> {
    let summary = {
        let conn = state.db.lock().map_err(lock_message)?;
        insert(&conn, drafts, project_id.as_deref(), now()).map_err(to_message)?
    };
    if summary.created > 0 {
        emit_items_changed(app, summary.ids.clone());
    }
    Ok(summary)
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TextFormat {
    Markdown,
    Text,
    Todoist,
}

/// Imports the contents of a file the user picked in the UI.
#[tauri::command]
pub async fn import_text(
    app: AppHandle,
    state: State<'_, AppState>,
    format: TextFormat,
    content: String,
    project_id: Option<String>,
) -> Result<ImportSummary, String> {
    let drafts = match format {
        TextFormat::Markdown | TextFormat::Text => parse_notes(&content),
        TextFormat::Todoist => parse_todoist(&content)?,
    };
    save(&app, &state, drafts, project_id)
}

/// Folders (Notes) or lists (Reminders) to choose from.
#[tauri::command]
pub async fn import_apple_sources(source: apple::Source) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || apple::sources(source))
        .await
        .map_err(|e| e.to_string())?
}

/// Reads one Notes folder or Reminders list (read-only) and imports it.
#[tauri::command]
pub async fn import_apple(
    app: AppHandle,
    state: State<'_, AppState>,
    source: apple::Source,
    name: String,
    project_id: Option<String>,
) -> Result<ImportSummary, String> {
    let drafts = tauri::async_runtime::spawn_blocking(move || apple::read(source, &name))
        .await
        .map_err(|e| e.to_string())??;
    save(&app, &state, drafts, project_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_makes_one_card_per_item_or_paragraph() {
        let md = "# Groceries\n\n- [ ] milk\n- [x] eggs\n  free range\n* bread\n\nCall the bank\nabout the card\n\n1. first\n";
        let d = parse_notes(md);
        let titles: Vec<&str> = d.iter().map(|d| d.title.as_str()).collect();
        assert_eq!(titles, ["milk", "eggs", "bread", "Call the bank", "first"]);
        assert_eq!(d[0].kind, ItemType::Task);
        assert!(d[1].done);
        assert_eq!(d[1].body.as_deref(), Some("eggs\nfree range"));
        assert_eq!(d[2].kind, ItemType::Note);
        assert_eq!(d[3].body.as_deref(), Some("Call the bank\nabout the card"));
    }

    #[test]
    fn todoist_rows_become_tasks() {
        let csv = "TYPE,CONTENT,DESCRIPTION,PRIORITY,INDENT,DATE\r\n\
                   section,Work,,,,\r\n\
                   task,\"Send \"\"Q3\"\" deck\",\"two\nlines\",1,1,2026-11-20\r\n\
                   task,Water plants,,4,1,every monday\r\n";
        let d = parse_todoist(csv).unwrap();
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].title, "Send \"Q3\" deck");
        assert_eq!(d[0].priority, Some(Priority::Urgent));
        assert!(d[0].due_at.is_some());
        assert!(d[0].body.as_deref().unwrap().contains("two\nlines"));
        assert!(d[1].body.as_deref().unwrap().contains("every monday"));
        assert!(parse_todoist("a,b\n1,2\n").is_err());
    }

    #[test]
    fn insert_files_cards_in_the_inbox_or_a_project() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let p = db::projects::create_project(&conn, "Home", "#fff", now).unwrap();
        let s = insert(&conn, parse_notes("- a\n- b"), Some(&p.id), now).unwrap();
        assert_eq!(s.created, 2);
        let item = db::items::get_item(&conn, &s.ids[0]).unwrap().unwrap();
        assert_eq!(item.project_id.as_deref(), Some(p.id.as_str()));
        assert!(insert(&conn, parse_notes("x"), Some("nope"), now).is_err());
    }
}
