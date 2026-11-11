//! Tag reuse during capture: the model sees the user's existing tags and may
//! apply them; it never invents new ones (a 4B model would scatter near-duplicate
//! tags like "work" / "lavoro" / "job" across the library).

use rusqlite::Connection;
use serde_json::{Value, json};

use crate::db::DbResult;

/// Tags offered to the model, most used first.
const MAX_OFFERED: u32 = 40;
/// Tags per card.
const MAX_PER_CARD: usize = 2;

/// The user's tags in use, most used first.
pub fn known(conn: &Connection) -> DbResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT t.name FROM tags t JOIN item_tags it ON it.tag_id = t.id \
         GROUP BY t.id ORDER BY COUNT(*) DESC, t.name COLLATE NOCASE LIMIT ?1",
    )?;
    let rows = stmt.query_map([MAX_OFFERED], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Adds the tag list to the extraction prompt, just above the note.
pub fn with_prompt(user: String, tags: &[String]) -> String {
    if tags.is_empty() {
        return user;
    }
    let line = format!(
        "Tags: add up to {MAX_PER_CARD} of these existing tags in \"tags\", only when one clearly fits: {}\n",
        tags.join(", ")
    );
    match user.rfind("Note: ") {
        Some(at) => format!("{}{line}{}", &user[..at], &user[at..]),
        None => format!("{line}{user}"),
    }
}

/// Lets each card carry `tags`, limited to the user's existing ones.
pub fn with_schema(mut schema: Value, tags: &[String]) -> Value {
    if tags.is_empty() {
        return schema;
    }
    schema["properties"]["items"]["items"]["properties"]["tags"] = json!({
        "type": "array",
        "maxItems": MAX_PER_CARD,
        "items": { "type": "string", "enum": tags },
    });
    schema
}

fn fold(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Levenshtein distance, capped early: only "is it ≤ 1" matters here.
fn within_one(a: &str, b: &str) -> bool {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.len().abs_diff(b.len()) > 1 {
        return false;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()] <= 1
}

/// The existing tag a model tag means: exact (ignoring case, '#', spaces and
/// punctuation), or one edit away for words of 5+ letters ("meeting"/"meetings").
pub fn closest<'a>(tag: &str, known: &'a [String]) -> Option<&'a String> {
    let t = fold(tag);
    if t.is_empty() {
        return None;
    }
    known.iter().find(|k| fold(k) == t).or_else(|| {
        known
            .iter()
            .find(|k| t.chars().count() >= 5 && within_one(&fold(k), &t))
    })
}

/// Maps model tags onto existing ones (keeping their spelling) and drops the rest.
pub fn snap(tags: &[String], known: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in tags {
        if let Some(k) = closest(t, known)
            && !out.contains(k)
        {
            out.push(k.clone());
        }
    }
    out.truncate(MAX_PER_CARD);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn snaps_to_existing_spelling_and_drops_invented_tags() {
        let known = v(&["Research", "meetings", "home"]);
        assert_eq!(
            snap(&v(&["research", "meeting", "travel"]), &known),
            v(&["Research", "meetings"])
        );
        // Short tags must match exactly: "hom" is not "home".
        assert!(snap(&v(&["hom"]), &known).is_empty());
    }

    #[test]
    fn prompt_and_schema_only_change_when_there_are_tags() {
        assert_eq!(
            with_prompt("Now: x\nNote: hi".into(), &[]),
            "Now: x\nNote: hi"
        );
        let p = with_prompt("Now: x\nNote: hi".into(), &v(&["home"]));
        assert!(p.contains("fits: home\nNote: hi"));
        let s = with_schema(
            json!({"properties":{"items":{"items":{"properties":{}}}}}),
            &v(&["home"]),
        );
        assert_eq!(
            s["properties"]["items"]["items"]["properties"]["tags"]["items"]["enum"][0],
            "home"
        );
    }
}
