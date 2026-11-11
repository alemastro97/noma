//! Linking a freshly captured card to cards it is clearly related to.
//!
//! Deliberately lexical and local: shared specific words between the new card
//! (title and what the user said) and an existing card's title. No extra model
//! call, so it costs nothing in latency, and it errs on the side of linking
//! too little — a wrong link is more annoying than a missing one.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::db::{self, DbResult};

const MAX_LINKS: usize = 2;
/// A single shared word links only when it is this long (specific enough).
const SOLE_WORD_LEN: usize = 7;
/// Words in this many existing titles are too common to mean anything.
const COMMON_IN: usize = 5;

const STOP: &[&str] = &[
    // English
    "this",
    "that",
    "with",
    "from",
    "have",
    "will",
    "would",
    "should",
    "could",
    "about",
    "your",
    "their",
    "there",
    "what",
    "when",
    "then",
    "than",
    "them",
    "they",
    "into",
    "also",
    "just",
    "need",
    "make",
    "call",
    "send",
    "check",
    "remind",
    "remember",
    "tomorrow",
    "today",
    "next",
    "week",
    "month",
    "year",
    "morning",
    "evening",
    "afternoon",
    "idea",
    "note",
    "task",
    // Italian
    "della",
    "delle",
    "degli",
    "dello",
    "dell",
    "alla",
    "alle",
    "agli",
    "allo",
    "nella",
    "nelle",
    "sono",
    "come",
    "anche",
    "perch",
    "questo",
    "questa",
    "questi",
    "quello",
    "quella",
    "devo",
    "dovrei",
    "ricordami",
    "ricordare",
    "chiamare",
    "domani",
    "oggi",
    "stasera",
    "mattina",
    "pomeriggio",
    "sera",
    "settimana",
    "prossima",
    "prossimo",
    "fare",
    "cosa",
    "cose",
    "molto",
    "ancora",
    "quando",
    "dopo",
    "prima",
    "ogni",
    "idea",
    "nota",
];

/// Lowercased, accent-preserving words of at least 4 letters, reduced to a
/// crude 5-letter stem so "fotografo" and "fotografi" meet. Stop words drop out.
pub fn tokens(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 4 && !STOP.contains(w))
        .map(|w| {
            if w.chars().count() > 5 {
                w.chars().take(5).collect()
            } else {
                w.to_string()
            }
        })
        .collect()
}

struct Row {
    id: String,
    title: String,
    project_id: Option<String>,
}

/// Links each of `ids` to up to two clearly related existing cards. Cards from
/// the same capture, project cards, done cards and capture placeholders are
/// never candidates. Returns how many links were created.
pub fn link_related(conn: &Connection, ids: &[String], now: DateTime<Utc>) -> DbResult<usize> {
    let mut stmt = conn.prepare(
        "SELECT i.id, i.title, i.project_id FROM items i
         WHERE i.status != 'done' AND i.kind != 'project_ref'
           AND i.archived_at IS NULL AND i.deleted_at IS NULL
           AND NOT EXISTS (SELECT 1 FROM captures k WHERE k.item_id = i.id AND k.status IN ('pending', 'processing'))",
    )?;
    let rows: Vec<Row> = stmt
        .query_map([], |r| {
            Ok(Row {
                id: r.get(0)?,
                title: r.get(1)?,
                project_id: r.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    let skip: HashSet<&str> = ids.iter().map(String::as_str).collect();
    let candidates: Vec<(&Row, HashSet<String>)> = rows
        .iter()
        .filter(|r| !skip.contains(r.id.as_str()))
        .map(|r| (r, tokens(&r.title)))
        .collect();

    let mut frequency: HashMap<&str, usize> = HashMap::new();
    for (_, toks) in &candidates {
        for t in toks {
            *frequency.entry(t.as_str()).or_default() += 1;
        }
    }

    let mut created = 0;
    for id in ids {
        let Some(item) = db::items::get_item(conn, id)? else {
            continue;
        };
        let mut mine = tokens(&item.title);
        mine.extend(tokens(item.body.as_deref().unwrap_or("")));

        let mut scored: Vec<(&Row, usize, usize)> = candidates
            .iter()
            .filter_map(|(row, toks)| {
                let shared: Vec<&String> = toks
                    .iter()
                    .filter(|t| mine.contains(*t))
                    .filter(|t| frequency.get(t.as_str()).copied().unwrap_or(0) < COMMON_IN)
                    .collect();
                let longest = shared.iter().map(|t| t.chars().count()).max().unwrap_or(0);
                // The stem is capped at 5 letters, so judge "specific" by the original title word.
                let specific = row
                    .title
                    .to_lowercase()
                    .split(|c: char| !c.is_alphanumeric())
                    .any(|w| {
                        w.chars().count() >= SOLE_WORD_LEN
                            && shared.iter().any(|s| w.starts_with(s.as_str()))
                    });
                let same_project = item.project_id.is_some() && item.project_id == row.project_id;
                let ok = shared.len() >= 2 || (shared.len() == 1 && specific);
                // Same project: one shared 5-letter stem is enough context.
                let ok = ok || (same_project && !shared.is_empty() && longest >= 5);
                ok.then_some((*row, shared.len(), longest))
            })
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
        for (row, _, _) in scored.into_iter().take(MAX_LINKS) {
            db::links::create_link(conn, id, &row.id, None, now)?;
            created += 1;
        }
    }
    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{ItemType, NewItem};

    fn card(conn: &Connection, title: &str, body: Option<&str>) -> String {
        db::items::create_item(
            conn,
            NewItem {
                kind: ItemType::Task,
                title: title.into(),
                body: body.map(str::to_string),
                ..Default::default()
            },
            Utc::now(),
        )
        .unwrap()
        .id
    }

    fn linked(conn: &Connection, a: &str, b: &str) -> bool {
        conn.query_row(
            "SELECT COUNT(*) FROM connections WHERE (from_item=?1 AND to_item=?2) OR (from_item=?2 AND to_item=?1)",
            [a, b],
            |r| r.get::<_, i64>(0),
        )
        .unwrap()
            > 0
    }

    #[test]
    fn stems_meet_and_stop_words_drop() {
        let t = tokens("Il fotografo per la sessione");
        assert!(t.contains("fotog"));
        assert!(!t.contains("della"));
        assert_eq!(tokens("Dentist"), tokens("dentist"));
    }

    #[test]
    fn links_cards_sharing_specific_words() {
        let conn = db::open_in_memory().unwrap();
        let photographer = card(&conn, "Book the photographer for the shoot", None);
        let milk = card(&conn, "Buy milk", None);
        let venue = card(&conn, "Venue deposit with Luca", None);
        let new = card(
            &conn,
            "Photographer contract",
            Some("remind me to send the photographer contract"),
        );
        let n = link_related(&conn, std::slice::from_ref(&new), Utc::now()).unwrap();
        assert_eq!(n, 1);
        assert!(linked(&conn, &new, &photographer));
        assert!(!linked(&conn, &new, &milk));
        assert!(!linked(&conn, &new, &venue));
    }

    #[test]
    fn unrelated_cards_and_same_capture_siblings_are_not_linked() {
        let conn = db::open_in_memory().unwrap();
        let a = card(&conn, "Pay the internet bill", None);
        let b = card(&conn, "Pay the electricity bill", None);
        // Two cards from one capture never link to each other.
        assert_eq!(
            link_related(&conn, &[a.clone(), b.clone()], Utc::now()).unwrap(),
            0
        );
        let c = card(&conn, "Walk the dog", None);
        assert_eq!(link_related(&conn, &[c], Utc::now()).unwrap(), 0);
    }
}
