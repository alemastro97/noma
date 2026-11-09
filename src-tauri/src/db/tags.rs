use rusqlite::{Connection, OptionalExtension, params};

use super::error::DbResult;
use super::models::{Item, new_id};

/// Normalizes user tag input: trims, strips a leading '#', drops blanks and
/// case-insensitive duplicates, keeps the first spelling.
pub fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in tags {
        let t = raw.trim().trim_start_matches('#').trim().to_string();
        if t.is_empty() {
            continue;
        }
        if !out.iter().any(|e| e.eq_ignore_ascii_case(&t)) {
            out.push(t);
        }
    }
    out
}

/// Replaces the tags of `item_id` with `tags`, creating tags on first use.
pub fn set_item_tags(conn: &Connection, item_id: &str, tags: &[String]) -> DbResult<Vec<String>> {
    let tags = normalize_tags(tags);
    conn.execute("DELETE FROM item_tags WHERE item_id = ?1", [item_id])?;
    for name in &tags {
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
                [name],
                |r| r.get(0),
            )
            .optional()?;
        let tag_id = match existing {
            Some(id) => id,
            None => {
                let id = new_id();
                conn.execute(
                    "INSERT INTO tags (id, name) VALUES (?1, ?2)",
                    params![id, name],
                )?;
                id
            }
        };
        conn.execute(
            "INSERT OR IGNORE INTO item_tags (item_id, tag_id) VALUES (?1, ?2)",
            params![item_id, tag_id],
        )?;
    }
    Ok(tags)
}

/// Fills `item.tags` for each item.
pub fn attach_tags(conn: &Connection, items: &mut [Item]) -> DbResult<()> {
    let mut stmt = conn.prepare_cached(
        "SELECT t.name FROM item_tags it JOIN tags t ON t.id = it.tag_id \
         WHERE it.item_id = ?1 ORDER BY t.name COLLATE NOCASE",
    )?;
    for item in items.iter_mut() {
        let names = stmt.query_map([&item.id], |r| r.get::<_, String>(0))?;
        item.tags = names.collect::<Result<_, _>>()?;
    }
    Ok(())
}

/// All tag names in use, for autocomplete.
pub fn list_tags(conn: &Connection) -> DbResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT t.name FROM tags t JOIN item_tags it ON it.tag_id = t.id ORDER BY t.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{self, models::*};
    use chrono::Utc;

    #[test]
    fn normalize_strips_hash_and_dedupes() {
        let out = normalize_tags(&[
            "#research".into(),
            "Research".into(),
            "  voice ".into(),
            "#".into(),
        ]);
        assert_eq!(out, vec!["research".to_string(), "voice".to_string()]);
    }

    #[test]
    fn set_and_attach_roundtrip() {
        let conn = db::open_in_memory().unwrap();
        let item = db::items::create_item(
            &conn,
            NewItem {
                title: "Note".into(),
                tags: Some(vec!["#voice".into(), "research".into()]),
                ..Default::default()
            },
            Utc::now(),
        )
        .unwrap();
        assert_eq!(item.tags, vec!["research".to_string(), "voice".to_string()]);
        set_item_tags(&conn, &item.id, &["voice".into()]).unwrap();
        let again = db::items::get_item(&conn, &item.id).unwrap().unwrap();
        assert_eq!(again.tags, vec!["voice".to_string()]);
        assert_eq!(list_tags(&conn).unwrap(), vec!["voice".to_string()]);
    }
}
