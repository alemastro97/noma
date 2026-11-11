//! The semantic index: one f32 embedding per card. See docs/specs/semantic.md.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};

use super::DbResult;

/// The text a card contributes to the index.
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    pub id: String,
    pub title: String,
    pub body: Option<String>,
}

/// Every card, title and body (project reference cards included: their names matter).
pub fn sources(conn: &Connection) -> DbResult<Vec<Source>> {
    let mut stmt =
        conn.prepare_cached("SELECT id, title, body FROM items ORDER BY updated_at DESC")?;
    let rows = stmt.query_map([], |r| {
        Ok(Source {
            id: r.get(0)?,
            title: r.get(1)?,
            body: r.get(2)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// `item_id → content_hash` for the rows embedded with `model_id`.
pub fn hashes(conn: &Connection, model_id: &str) -> DbResult<HashMap<String, String>> {
    let mut stmt =
        conn.prepare_cached("SELECT item_id, content_hash FROM embeddings WHERE model_id = ?1")?;
    let rows = stmt.query_map([model_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Stores a card's embedding. A card deleted meanwhile is skipped silently.
pub fn upsert(
    conn: &Connection,
    item_id: &str,
    model_id: &str,
    content_hash: &str,
    vector: &[f32],
    now: DateTime<Utc>,
) -> DbResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO embeddings (item_id, model_id, content_hash, dims, vector, updated_at) \
         SELECT ?1, ?2, ?3, ?4, ?5, ?6 WHERE EXISTS (SELECT 1 FROM items WHERE id = ?1)",
        params![
            item_id,
            model_id,
            content_hash,
            vector.len() as i64,
            encode(vector),
            now.to_rfc3339()
        ],
    )?;
    Ok(())
}

/// All vectors embedded with `model_id`.
pub fn vectors(conn: &Connection, model_id: &str) -> DbResult<Vec<(String, Vec<f32>)>> {
    let mut stmt =
        conn.prepare_cached("SELECT item_id, vector FROM embeddings WHERE model_id = ?1")?;
    let rows = stmt.query_map([model_id], |r| {
        let blob: Vec<u8> = r.get(1)?;
        Ok((r.get::<_, String>(0)?, decode(&blob)))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Number of cards embedded with `model_id`.
pub fn count(conn: &Connection, model_id: &str) -> DbResult<u32> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM embeddings WHERE model_id = ?1",
        [model_id],
        |r| r.get(0),
    )?)
}

/// Drops rows of other models (after a model change) and of vanished cards.
pub fn prune(conn: &Connection, model_id: &str) -> DbResult<()> {
    conn.execute(
        "DELETE FROM embeddings WHERE model_id != ?1 OR item_id NOT IN (SELECT id FROM items)",
        [model_id],
    )?;
    Ok(())
}

/// Empties the index (model removed or rebuild requested).
pub fn clear(conn: &Connection) -> DbResult<()> {
    conn.execute("DELETE FROM embeddings", [])?;
    Ok(())
}

/// Full-text fallback for questions: cards matching ANY meaningful word,
/// best first. Used by Ask NOMA when there is no embedding index yet.
pub fn keyword_ids(conn: &Connection, text: &str, limit: u32) -> DbResult<Vec<String>> {
    let terms: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 4)
        .take(12)
        .map(|w| format!("\"{}\"*", w.to_lowercase()))
        .collect();
    if terms.is_empty() {
        return Ok(vec![]);
    }
    let mut stmt = conn.prepare_cached(
        "SELECT id FROM items_fts WHERE items_fts MATCH ?1 ORDER BY bm25(items_fts) LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![terms.join(" OR "), limit], |r| r.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Little-endian f32 blob.
pub fn encode(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

pub fn decode(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{
        self,
        models::{ItemType, NewItem},
    };

    fn add(conn: &Connection, title: &str) -> String {
        db::items::create_item(
            conn,
            NewItem {
                kind: ItemType::Note,
                title: title.into(),
                ..Default::default()
            },
            Utc::now(),
        )
        .unwrap()
        .id
    }

    #[test]
    fn blob_roundtrip() {
        let v = vec![0.25, -1.5, 3.0];
        assert_eq!(decode(&encode(&v)), v);
    }

    #[test]
    fn upsert_count_prune_and_cascade() {
        let conn = db::open_in_memory().unwrap();
        let a = add(&conn, "Book the photographer");
        let b = add(&conn, "Buy film");
        upsert(&conn, &a, "m1", "h1", &[1.0, 0.0], Utc::now()).unwrap();
        upsert(&conn, &b, "m0", "h2", &[0.0, 1.0], Utc::now()).unwrap();
        upsert(&conn, "missing", "m1", "h3", &[0.0, 1.0], Utc::now()).unwrap();
        assert_eq!(count(&conn, "m1").unwrap(), 1);
        prune(&conn, "m1").unwrap();
        assert_eq!(count(&conn, "m0").unwrap(), 0);
        assert_eq!(hashes(&conn, "m1").unwrap().get(&a).unwrap(), "h1");
        db::items::delete_item(&conn, &a).unwrap();
        assert_eq!(count(&conn, "m1").unwrap(), 0);
    }

    #[test]
    fn keyword_fallback_matches_any_word() {
        let conn = db::open_in_memory().unwrap();
        let a = add(&conn, "Photographer for the launch");
        add(&conn, "Groceries");
        let ids = keyword_ids(&conn, "who is the photographer?", 5).unwrap();
        assert_eq!(ids, vec![a]);
    }
}
