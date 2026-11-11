//! Keeping things organized after capture: re-sorting the Inbox, duplicate
//! detection, breaking tasks down, project briefs, the weekly review and tag
//! reuse. See docs/specs/organize.md.

pub mod breakdown;
pub mod briefs;
pub mod commands;
pub mod duplicates;
pub mod resort;
pub mod review;
pub mod tags;

use serde::de::DeserializeOwned;

use crate::ai::AiError;

/// Parses a model reply into `T`, tolerating Markdown fences.
pub fn parse_reply<T: DeserializeOwned>(text: &str) -> Result<T, AiError> {
    let t = text.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t).trim();
    serde_json::from_str(t).map_err(|e| {
        let head: String = t.chars().take(200).collect();
        AiError::BadOutput(format!("{e}: {head}"))
    })
}

/// One card as a compact prompt line: title plus what matters for planning.
pub fn card_line(item: &crate::db::models::Item, tz: chrono_tz::Tz) -> String {
    use crate::db::models::{Priority, Status};
    let mut extra = Vec::new();
    if let Some(d) = item.due_at.or(item.remind_at) {
        extra.push(format!("due {}", d.with_timezone(&tz).format("%Y-%m-%d")));
    }
    match item.priority {
        Priority::Urgent => extra.push("urgent".into()),
        Priority::High => extra.push("important".into()),
        _ => {}
    }
    if item.status == Status::Doing {
        extra.push("in progress".into());
    }
    let title: String = item.title.chars().take(90).collect();
    if extra.is_empty() {
        title
    } else {
        format!("{title} ({})", extra.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    struct S {
        summary: String,
    }

    #[test]
    fn replies_parse_with_or_without_fences() {
        let a: S = parse_reply("```json\n{\"summary\":\"ok\"}\n```").unwrap();
        assert_eq!(a.summary, "ok");
        assert!(parse_reply::<S>("nope").is_err());
    }
}
