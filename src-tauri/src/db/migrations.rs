use rusqlite::Connection;

use super::error::{DbError, DbResult};

/// Ordered migrations. Each entry is applied exactly once; the index + 1 is its version.
const MIGRATIONS: &[&str] = &[
    include_str!("migrations/0001_init.sql"),
    include_str!("migrations/0002_events_end.sql"),
    include_str!("migrations/0003_capture_hints.sql"),
    include_str!("migrations/0004_reminders.sql"),
    include_str!("migrations/0005_filing_examples.sql"),
    include_str!("migrations/0009_workspace.sql"),
];

/// Applies any pending migrations and returns the schema version afterwards.
pub fn migrate(conn: &mut Connection) -> DbResult<u32> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);")?;
    let current: u32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )
        .map_err(|e| DbError::Migration(e.to_string()))?;

    for (index, sql) in MIGRATIONS.iter().enumerate() {
        let version = index as u32 + 1;
        if version <= current {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(sql)
            .map_err(|e| DbError::Migration(format!("v{version}: {e}")))?;
        tx.execute("INSERT INTO schema_version(version) VALUES (?1)", [version])?;
        tx.commit()?;
    }
    Ok(MIGRATIONS.len() as u32)
}

#[cfg(test)]
mod tests {
    use crate::db;

    #[test]
    fn migrate_is_idempotent() {
        let mut conn = db::open_in_memory().unwrap();
        assert_eq!(db::migrate(&mut conn).unwrap(), 5);
        assert_eq!(db::migrate(&mut conn).unwrap(), 5);
        let rows: u32 = conn
            .query_row("SELECT COUNT(*) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 5);
    }

    #[test]
    fn open_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("deeper").join("noma.db");
        let conn = db::open(&path).unwrap();
        assert!(path.exists());
        let fk: u32 = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }
}
