//! Duplicate detection for fresh captures, and merging a duplicate into the
//! card it repeats. Lexical like `ai::related` (no model call): titles that
//! share most of their specific words, or are the same words outright.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use rusqlite::Connection;

use crate::ai::related::tokens;
use crate::db::{
    self, DbError, DbResult,
    models::{Item, ItemPatch},
};

/// Share of distinct title stems two cards must have in common.
const MIN_OVERLAP: f64 = 0.6;

fn plain(s: &str) -> String {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// How alike two titles are, 0..=1. Same words = 1.
pub fn similarity(a: &str, b: &str) -> f64 {
    if !plain(a).is_empty() && plain(a) == plain(b) {
        return 1.0;
    }
    let (ta, tb) = (tokens(a), tokens(b));
    let shared = ta.intersection(&tb).count();
    if shared < 2 {
        return 0.0;
    }
    shared as f64 / ta.union(&tb).count() as f64
}

/// The open card `item` most looks like, if any is close enough.
fn best_match<'a>(item: &Item, candidates: &'a [(String, String)]) -> Option<&'a (String, String)> {
    candidates
        .iter()
        .filter(|(id, _)| *id != item.id)
        .map(|c| (c, similarity(&item.title, &c.1)))
        .filter(|(_, s)| *s >= MIN_OVERLAP)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(c, _)| c)
}

/// Marks each of `ids` (cards just written by a capture) that closely matches
/// an existing open card with a "Looks like …" hint. Cards from the same capture
/// are never compared with each other. Returns how many were marked.
pub fn mark(conn: &Connection, ids: &[String]) -> DbResult<usize> {
    let skip: HashSet<&str> = ids.iter().map(String::as_str).collect();
    let mut stmt = conn.prepare(&format!(
        "SELECT i.id, i.title FROM items i
         WHERE i.status != 'done' AND i.kind != 'project_ref' AND {}
           AND NOT EXISTS (SELECT 1 FROM captures k WHERE k.item_id = i.id AND k.status IN ('pending', 'processing'))",
        db::items::VISIBLE
    ))?;
    let candidates: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|(id, _): &(String, String)| !skip.contains(id.as_str()))
        .collect();
    let mut marked = 0;
    for id in ids {
        let Some(item) = db::items::get_item(conn, id)? else {
            continue;
        };
        let Some((other_id, other_title)) = best_match(&item, &candidates) else {
            continue;
        };
        let mut hint = item.hint.clone().unwrap_or_default();
        hint.similar_to = Some(other_id.clone());
        hint.similar_title = Some(other_title.clone());
        let patch = ItemPatch {
            hint: Some(Some(hint)),
            ..Default::default()
        };
        db::items::update_item(conn, id, patch, Utc::now(), chrono_tz::UTC)?;
        marked += 1;
    }
    Ok(marked)
}

/// Folds `duplicate` into `into`: its title and notes are appended to the
/// notes, tags are united, missing dates are taken over and its links move to
/// `into`. The duplicate is then deleted. Returns the merged card.
pub fn merge(
    conn: &Connection,
    duplicate: &str,
    into: &str,
    now: DateTime<Utc>,
    tz: Tz,
) -> DbResult<Item> {
    if duplicate == into {
        return Err(DbError::Invalid(
            "a card cannot be merged into itself".into(),
        ));
    }
    let dup = db::items::get_item(conn, duplicate)?
        .ok_or_else(|| DbError::NotFound(format!("item {duplicate}")))?;
    let target = db::items::get_item(conn, into)?
        .ok_or_else(|| DbError::NotFound(format!("item {into}")))?;

    let mut extra = Vec::new();
    if plain(&dup.title) != plain(&target.title) {
        extra.push(dup.title.clone());
    }
    if let Some(b) = dup.body.as_deref().map(str::trim).filter(|b| !b.is_empty())
        && !target.body.as_deref().unwrap_or("").contains(b)
    {
        extra.push(b.to_string());
    }
    let body = match (target.body.as_deref().map(str::trim), extra.is_empty()) {
        (_, true) => None,
        (Some(t), false) if !t.is_empty() => Some(Some(format!("{t}\n\n{}", extra.join("\n")))),
        _ => Some(Some(extra.join("\n"))),
    };
    let mut tags = target.tags.clone();
    for t in &dup.tags {
        if !tags.iter().any(|x| x.eq_ignore_ascii_case(t)) {
            tags.push(t.clone());
        }
    }
    let patch = ItemPatch {
        body,
        tags: Some(tags),
        due_at: (target.due_at.is_none() && dup.due_at.is_some()).then_some(dup.due_at),
        remind_at: (target.remind_at.is_none() && dup.remind_at.is_some()).then_some(dup.remind_at),
        ..Default::default()
    };

    // Not one transaction: `update_item` opens its own. The order keeps a
    // failure harmless (the duplicate is deleted last).
    let neighbours: Vec<(String, Option<String>)> = {
        let mut stmt = conn.prepare(
            "SELECT CASE WHEN from_item = ?1 THEN to_item ELSE from_item END, label \
             FROM connections WHERE from_item = ?1 OR to_item = ?1",
        )?;
        stmt.query_map([duplicate], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?
    };
    for (other, label) in neighbours {
        if other != into {
            db::links::create_link(conn, into, &other, label.as_deref(), now)?;
        }
    }
    db::items::update_item(conn, into, patch, now, tz)?;
    db::items::delete_item(conn, duplicate)?;
    db::items::get_item(conn, into)?.ok_or_else(|| DbError::NotFound(format!("item {into}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::NewItem;

    fn card(conn: &Connection, title: &str, body: Option<&str>) -> String {
        db::items::create_item(
            conn,
            NewItem {
                title: title.into(),
                body: body.map(str::to_string),
                ..Default::default()
            },
            Utc::now(),
        )
        .unwrap()
        .id
    }

    #[test]
    fn near_identical_titles_are_flagged_and_unrelated_ones_are_not() {
        assert_eq!(similarity("Dentist", "dentist!"), 1.0);
        assert!(similarity("Book the photographer", "Book photographer for launch") >= 0.6);
        assert_eq!(similarity("Pay the internet bill", "Walk the dog"), 0.0);

        let conn = db::open_in_memory().unwrap();
        let old = card(&conn, "Renew the passport", None);
        card(&conn, "Buy milk", None);
        let new = card(&conn, "Renew passport", None);
        assert_eq!(mark(&conn, std::slice::from_ref(&new)).unwrap(), 1);
        let hint = db::items::get_item(&conn, &new)
            .unwrap()
            .unwrap()
            .hint
            .unwrap();
        assert_eq!(hint.similar_to.as_deref(), Some(old.as_str()));
    }

    #[test]
    fn merge_keeps_the_words_and_removes_the_duplicate() {
        let conn = db::open_in_memory().unwrap();
        let keep = card(&conn, "Renew the passport", Some("Office opens at 9"));
        let dup = card(&conn, "Renew passport before May", Some("bring two photos"));
        let merged = merge(&conn, &dup, &keep, Utc::now(), chrono_tz::UTC).unwrap();
        let body = merged.body.unwrap();
        assert!(body.starts_with("Office opens at 9"));
        assert!(body.contains("bring two photos"));
        assert!(db::items::get_item(&conn, &dup).unwrap().is_none());
    }
}
