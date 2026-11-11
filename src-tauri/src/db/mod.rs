//! SQLite data layer. The frontend never touches SQL: Tauri commands call these
//! functions and emit change events.

pub mod boards;
pub mod captures;
pub mod embeddings;
pub mod error;
pub mod filing;
pub mod items;
pub mod links;
pub mod migrations;
pub mod models;
pub mod placements;
pub mod projects;
pub mod recurrence;
pub mod reminders;
#[cfg(debug_assertions)]
pub mod seed;
pub mod settings;
pub mod tags;

use std::path::Path;

use rusqlite::Connection;

pub use error::{DbError, DbResult};
pub use migrations::migrate;

fn configure(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(())
}

/// Opens (creating parent directories and the file if needed) and migrates the database.
pub fn open(path: &Path) -> DbResult<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    migrate(&mut conn)?;
    Ok(conn)
}

/// An in-memory, fully migrated database (tests).
pub fn open_in_memory() -> DbResult<Connection> {
    let mut conn = Connection::open_in_memory()?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    migrate(&mut conn)?;
    Ok(conn)
}
