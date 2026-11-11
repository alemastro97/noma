//! Ask NOMA: a single question answered from the user's own cards, with
//! numbered citations. Prompt, output schema and parsing (pure).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::ai::AiError;
use crate::db::models::Item;

/// Cards given to the model per question.
pub const MAX_CARDS: usize = 8;
/// Earlier turns kept as context.
pub const MAX_HISTORY: usize = 3;
const MAX_BODY_CHARS: usize = 600;
const MAX_ANSWER_CHARS: usize = 1200;

pub const SYSTEM_PROMPT: &str = "You are NOMA, the user's second mind. You answer questions \
using ONLY the user's own cards listed below (tasks, reminders, notes, ideas, events). \
Rules: answer briefly (1-4 sentences) in the language of the question; cite the cards you \
used with their number in square brackets, like [2]; never invent facts, dates or names \
that are not in the cards; if the cards do not contain the answer, say so plainly. \
Put the numbers of the cards you used in `sources`.";

/// One earlier question and its answer.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Turn {
    pub question: String,
    pub answer: String,
}

fn card_line(n: usize, item: &Item) -> String {
    let mut line = format!(
        "[{n}] ({}, {}) {}",
        item.kind.as_str(),
        item.status.as_str(),
        item.title.trim()
    );
    if let Some(due) = item.due_at {
        line.push_str(&format!(" · due {}", due.format("%Y-%m-%d %H:%M UTC")));
    }
    if let Some(body) = item
        .body
        .as_deref()
        .map(str::trim)
        .filter(|b| !b.is_empty())
        && body != item.title.trim()
    {
        let body: String = body.chars().take(MAX_BODY_CHARS).collect();
        line.push_str(&format!("\n    {}", body.replace('\n', "\n    ")));
    }
    line
}

pub fn user_prompt(question: &str, history: &[Turn], cards: &[Item], today: &str) -> String {
    let mut out = format!("Today is {today}.\n\nCards:\n");
    for (i, item) in cards.iter().enumerate() {
        out.push_str(&card_line(i + 1, item));
        out.push('\n');
    }
    let recent = &history[history.len().saturating_sub(MAX_HISTORY)..];
    if !recent.is_empty() {
        out.push_str("\nEarlier in this conversation:\n");
        for t in recent {
            out.push_str(&format!(
                "Q: {}\nA: {}\n",
                t.question.trim(),
                t.answer.trim()
            ));
        }
    }
    out.push_str(&format!("\nQuestion: {}", question.trim()));
    out
}

pub fn schema(cards: usize) -> Value {
    json!({
        "type": "object",
        "properties": {
            "answer": { "type": "string", "maxLength": MAX_ANSWER_CHARS },
            "sources": {
                "type": "array",
                "items": { "type": "integer", "minimum": 1, "maximum": cards.max(1) },
                "maxItems": cards.max(1)
            }
        },
        "required": ["answer", "sources"],
        "additionalProperties": false
    })
}

/// The answer and the 1-based card numbers it cites (from `sources` and from
/// `[n]` markers in the text), deduplicated and in range.
pub fn parse(output: &str, cards: usize) -> Result<(String, Vec<u32>), AiError> {
    let v: Value = serde_json::from_str(output.trim())
        .map_err(|e| AiError::BadOutput(format!("answer is not JSON: {e}")))?;
    let answer = v["answer"]
        .as_str()
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .ok_or_else(|| AiError::BadOutput("empty answer".into()))?
        .to_string();
    let mut cited: Vec<u32> = Vec::new();
    let mut push = |n: u64| {
        if n >= 1 && (n as usize) <= cards && !cited.contains(&(n as u32)) {
            cited.push(n as u32);
        }
    };
    for n in markers(&answer) {
        push(n);
    }
    for n in v["sources"].as_array().into_iter().flatten() {
        if let Some(n) = n.as_u64() {
            push(n);
        }
    }
    Ok((answer, cited))
}

/// Numbers written as `[n]` in the answer, in order.
fn markers(text: &str) -> Vec<u64> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        rest = &rest[start + 1..];
        if let Some(end) = rest.find(']')
            && let Ok(n) = rest[..end].trim().parse::<u64>()
        {
            out.push(n);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_answer_and_citations() {
        let (a, cited) = parse(
            r#"{"answer":"Marco shoots on Monday [2], see also [9].","sources":[1,2]}"#,
            3,
        )
        .unwrap();
        assert!(a.starts_with("Marco"));
        assert_eq!(cited, vec![2, 1]);
        assert!(parse(r#"{"answer":"  ","sources":[]}"#, 3).is_err());
    }

    #[test]
    fn prompt_lists_cards_and_recent_history() {
        let item: Item = serde_json::from_value(json!({
            "id": "a", "kind": "task", "title": "Book photographer", "body": "Marco, Monday",
            "status": "next", "priority": "normal", "dueAt": null, "endsAt": null,
            "remindAt": null, "snoozedUntil": null, "recurrence": null, "projectId": null,
            "createdAt": "2026-11-11T10:00:00Z", "updatedAt": "2026-11-11T10:00:00Z",
            "completedAt": null
        }))
        .unwrap();
        let history: Vec<Turn> = (0..5)
            .map(|i| Turn {
                question: format!("q{i}"),
                answer: format!("a{i}"),
            })
            .collect();
        let p = user_prompt("Who shoots?", &history, &[item], "2026-11-11");
        assert!(p.contains("[1] (task, next) Book photographer"));
        assert!(p.contains("Marco, Monday"));
        assert!(!p.contains("q1") && p.contains("q4"));
        assert!(p.ends_with("Question: Who shoots?"));
    }
}
