//! Read-only access to Apple Notes and Reminders through JavaScript for
//! Automation. macOS asks the user once before NOMA can read either app.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::import::Draft;
use super::jxa;
use crate::db::models::ItemType;

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Notes,
    Reminders,
}

fn names_script(source: Source) -> &'static str {
    match source {
        Source::Notes => "JSON.stringify(Application('Notes').folders.name())",
        Source::Reminders => "JSON.stringify(Application('Reminders').lists.name())",
    }
}

fn read_script(source: Source, name: &str) -> String {
    // The name is embedded as a JSON string literal, which is valid JavaScript.
    let name = serde_json::to_string(name).unwrap_or_else(|_| "\"\"".into());
    match source {
        Source::Notes => format!(
            "const out = [];
             Application('Notes').folders.whose({{name: {name}}})().forEach(f => {{
               const t = f.notes.name(), b = f.notes.plaintext();
               t.forEach((title, i) => out.push({{title, body: b[i] || null, due: null}}));
             }});
             JSON.stringify(out)"
        ),
        Source::Reminders => format!(
            "const out = [];
             Application('Reminders').lists.whose({{name: {name}}})().forEach(l => {{
               const r = l.reminders.whose({{completed: false}});
               const t = r.name(), b = r.body(), d = r.dueDate();
               t.forEach((title, i) => out.push({{
                 title, body: b[i] || null, due: d[i] ? d[i].toISOString() : null
               }}));
             }});
             JSON.stringify(out)"
        ),
    }
}

#[derive(Deserialize, Debug)]
struct Raw {
    title: String,
    body: Option<String>,
    due: Option<DateTime<Utc>>,
}

/// Error code the UI turns into "allow NOMA in System Settings → Automation".
pub const AUTOMATION_DENIED: &str = "automation-denied";

fn friendly(err: String) -> String {
    if err.contains("-1743") || err.contains("Not authorized") {
        AUTOMATION_DENIED.to_string()
    } else {
        err
    }
}

/// Folder (Notes) or list (Reminders) names, without duplicates.
pub fn sources(source: Source) -> Result<Vec<String>, String> {
    let out = jxa(names_script(source)).map_err(|e| friendly(e))?;
    let mut names: Vec<String> = serde_json::from_str(&out).map_err(|e| e.to_string())?;
    let mut seen = std::collections::HashSet::new();
    names.retain(|n| seen.insert(n.clone()));
    Ok(names)
}

/// Turns raw notes / reminders into cards.
fn drafts(source: Source, raw: Vec<Raw>) -> Vec<Draft> {
    raw.into_iter()
        .filter_map(|r| {
            let title = r.title.trim();
            // A note's plain text repeats its title on the first line.
            let body = r.body.as_deref().map(str::trim).map(|b| {
                b.strip_prefix(title)
                    .map(str::trim_start)
                    .unwrap_or(b)
                    .to_string()
            });
            let text = match body.filter(|b| !b.is_empty()) {
                Some(b) => format!("{title}\n{b}"),
                None => title.to_string(),
            };
            let kind = match (source, r.due) {
                (Source::Notes, _) => ItemType::Note,
                (Source::Reminders, Some(_)) => ItemType::Reminder,
                (Source::Reminders, None) => ItemType::Task,
            };
            let mut d = Draft::from_text(kind, &text)?;
            d.remind_at = r.due;
            Some(d)
        })
        .collect()
}

/// Reads every note in a folder, or every open reminder in a list.
pub fn read(source: Source, name: &str) -> Result<Vec<Draft>, String> {
    let out = jxa(&read_script(source, name)).map_err(|e| friendly(e))?;
    let raw: Vec<Raw> = serde_json::from_str(&out).map_err(|e| e.to_string())?;
    Ok(drafts(source, raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_drop_the_repeated_title_and_reminders_keep_their_time() {
        let raw: Vec<Raw> = serde_json::from_str(
            r#"[{"title":"Trip","body":"Trip\nbook hotel","due":null},
                {"title":"Pay rent","body":null,"due":"2026-12-01T09:00:00.000Z"}]"#,
        )
        .unwrap();
        let notes = drafts(Source::Notes, raw);
        assert_eq!(notes[0].title, "Trip");
        assert_eq!(notes[0].body.as_deref(), Some("Trip\nbook hotel"));
        assert_eq!(notes[0].kind, ItemType::Note);
        let raw: Vec<Raw> = serde_json::from_str(
            r#"[{"title":"Pay rent","body":null,"due":"2026-12-01T09:00:00.000Z"}]"#,
        )
        .unwrap();
        let r = drafts(Source::Reminders, raw);
        assert_eq!(r[0].kind, ItemType::Reminder);
        assert!(r[0].remind_at.is_some());
        assert!(read_script(Source::Notes, "a\"b").contains(r#""a\"b""#));
    }
}
