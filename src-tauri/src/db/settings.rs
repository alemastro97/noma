use rusqlite::{Connection, OptionalExtension, params};

use super::error::DbResult;

pub fn get_setting(conn: &Connection, key: &str) -> DbResult<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
            r.get(0)
        })
        .optional()?)
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> DbResult<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn upsert_and_read() {
        let conn = db::open_in_memory().unwrap();
        assert_eq!(get_setting(&conn, "ai.provider").unwrap(), None);
        set_setting(&conn, "ai.provider", "builtin").unwrap();
        set_setting(&conn, "ai.provider", "ollama").unwrap();
        assert_eq!(
            get_setting(&conn, "ai.provider").unwrap().as_deref(),
            Some("ollama")
        );
    }
}
