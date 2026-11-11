//! Files attached to a card. The bytes live under `<app data>/attachments/<item id>/`;
//! the table keeps the original name, type, size and the relative path.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use super::error::{DbError, DbResult};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    pub item_id: String,
    pub name: String,
    /// Path relative to the attachments folder.
    pub file: String,
    pub mime: String,
    pub size: i64,
    pub created_at: DateTime<Utc>,
}

fn row_to_attachment(row: &Row<'_>) -> rusqlite::Result<Attachment> {
    Ok(Attachment {
        id: row.get("id")?,
        item_id: row.get("item_id")?,
        name: row.get("name")?,
        file: row.get("file")?,
        mime: row.get("mime")?,
        size: row.get("size")?,
        created_at: row.get("created_at")?,
    })
}

pub fn insert_attachment(conn: &Connection, a: &Attachment) -> DbResult<()> {
    conn.execute(
        "INSERT INTO attachments (id, item_id, name, file, mime, size, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            a.id,
            a.item_id,
            a.name,
            a.file,
            a.mime,
            a.size,
            a.created_at
        ],
    )?;
    Ok(())
}

pub fn list_attachments(conn: &Connection, item_id: &str) -> DbResult<Vec<Attachment>> {
    let mut stmt = conn
        .prepare_cached("SELECT * FROM attachments WHERE item_id = ?1 ORDER BY created_at ASC")?;
    let rows = stmt.query_map([item_id], row_to_attachment)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn get_attachment(conn: &Connection, id: &str) -> DbResult<Attachment> {
    conn.query_row(
        "SELECT * FROM attachments WHERE id = ?1",
        [id],
        row_to_attachment,
    )
    .optional()?
    .ok_or_else(|| DbError::NotFound(format!("attachment {id}")))
}

/// Removes the row and returns it, so the caller can delete the file.
pub fn delete_attachment(conn: &Connection, id: &str) -> DbResult<Attachment> {
    let a = get_attachment(conn, id)?;
    conn.execute("DELETE FROM attachments WHERE id = ?1", [id])?;
    Ok(a)
}

/// A file name safe to write inside the attachments folder: no directories,
/// no leading dots, no control characters.
pub fn safe_file_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let cleaned: String = base
        .chars()
        .filter(|c| !c.is_control() && *c != ':')
        .collect();
    let cleaned = cleaned.trim().trim_start_matches('.').to_string();
    if cleaned.is_empty() {
        "file".into()
    } else {
        cleaned.chars().take(120).collect()
    }
}

/// MIME type from the file extension (enough to tell images from documents).
pub fn mime_for(name: &str) -> &'static str {
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "heic" => "image/heic",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "txt" | "md" => "text/plain",
        "csv" => "text/csv",
        "json" => "application/json",
        "zip" => "application/zip",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        _ => "application/octet-stream",
    }
}

/// Decodes standard base64 (padding optional, whitespace ignored).
pub fn decode_base64(input: &str) -> DbResult<Vec<u8>> {
    fn value(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some(u32::from(c - b'A')),
            b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
            b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0u32;
    for &c in input.as_bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        let v = value(c).ok_or_else(|| DbError::Invalid("attachment data".into()))?;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip_known_values() {
        assert_eq!(decode_base64("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(decode_base64("aGk").unwrap(), b"hi");
        assert!(decode_base64("a*b").is_err());
    }

    #[test]
    fn file_names_cannot_escape_the_folder() {
        assert_eq!(safe_file_name("../../etc/passwd"), "passwd");
        assert_eq!(safe_file_name(".hidden"), "hidden");
        assert_eq!(safe_file_name(""), "file");
        assert_eq!(mime_for("Photo.JPG"), "image/jpeg");
    }
}
