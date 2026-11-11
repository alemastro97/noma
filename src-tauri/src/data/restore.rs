//! Restore the live database from one of the daily snapshots. A fresh
//! snapshot is taken first, so a restore can itself be undone.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::backup;
use crate::commands::{emit_items_changed, lock_message};
use crate::db;
use crate::state::AppState;

/// Emitted after the database file was swapped: windows reload everything.
pub const DATA_RESTORED: &str = "data:restored";

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub name: String,
    pub at: DateTime<Utc>,
    pub size_bytes: u64,
}

/// Snapshots, newest first.
pub fn list(dir: &Path) -> Vec<Snapshot> {
    backup::snapshots(dir)
        .into_iter()
        .filter_map(|(path, at)| {
            Some(Snapshot {
                name: path.file_name()?.to_string_lossy().into_owned(),
                at,
                size_bytes: fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
            })
        })
        .collect()
}

/// Only a plain snapshot file name inside the backups folder is accepted.
pub fn resolve(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let plain = !name.contains('/') && !name.contains('\\') && !name.contains("..");
    if !plain || !name.starts_with("noma-") || !name.ends_with(".db") {
        return Err(format!("not a backup: {name}"));
    }
    let path = dir.join(name);
    if !path.is_file() {
        return Err(format!("backup not found: {name}"));
    }
    Ok(path)
}

/// The file must be an intact SQLite database with NOMA's tables.
pub fn validate(path: &Path) -> Result<(), String> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| e.to_string())?;
    let check: String = conn
        .query_row("PRAGMA quick_check", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if check != "ok" {
        return Err(format!("backup is damaged: {check}"));
    }
    conn.query_row("SELECT COUNT(*) FROM items", [], |r| r.get::<_, i64>(0))
        .map_err(|_| "not a NOMA backup".to_string())?;
    Ok(())
}

/// Replaces the live database file with `staged` (a copy we own and may
/// move). The live connection is closed first and reopened (and migrated)
/// afterwards; on failure the original file is reopened.
pub fn swap(live: &mut Connection, staged: &Path) -> Result<(), String> {
    let db_path = live
        .path()
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .ok_or("the database has no file")?;
    let placeholder = Connection::open_in_memory().map_err(|e| e.to_string())?;
    let old = std::mem::replace(live, placeholder);
    // Closing the last connection checkpoints and removes the WAL.
    if let Err((conn, e)) = old.close() {
        *live = conn;
        return Err(e.to_string());
    }
    let moved = (|| -> std::io::Result<()> {
        for ext in ["db-wal", "db-shm"] {
            let side = db_path.with_extension(ext);
            if side.exists() {
                fs::remove_file(side)?;
            }
        }
        fs::rename(staged, &db_path)
    })();
    let reopened = db::open(&db_path).map_err(|e| e.to_string());
    match (moved, reopened) {
        (Ok(()), Ok(conn)) => {
            *live = conn;
            Ok(())
        }
        (Err(e), Ok(conn)) => {
            *live = conn;
            Err(e.to_string())
        }
        (_, Err(e)) => Err(e),
    }
}

#[tauri::command]
pub async fn backup_list(app: AppHandle) -> Result<Vec<Snapshot>, String> {
    Ok(list(&backup::dir(&app)?))
}

#[tauri::command]
pub async fn backup_restore(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    let dir = backup::dir(&app)?;
    let source = resolve(&dir, &name)?;
    validate(&source)?;
    {
        let mut conn = state.db.lock().map_err(lock_message)?;
        let db_path = conn
            .path()
            .map(PathBuf::from)
            .ok_or("the database has no file")?;
        // Stage the copy before the safety snapshot, which may prune `source`.
        let staged = db_path.with_extension("db.restoring");
        fs::copy(&source, &staged).map_err(|e| e.to_string())?;
        if let Err(e) = backup::snapshot(&conn, &dir, Utc::now()) {
            let _ = fs::remove_file(&staged);
            return Err(e);
        }
        swap(&mut conn, &staged)?;
    }
    emit_items_changed(&app, Vec::new());
    let _ = app.emit(DATA_RESTORED, ());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::NewItem;

    fn add(conn: &Connection, title: &str) {
        db::items::create_item(
            conn,
            NewItem {
                title: title.into(),
                ..Default::default()
            },
            Utc::now(),
        )
        .unwrap();
    }

    fn count(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn swap_replaces_the_live_database() {
        let tmp = tempfile::tempdir().unwrap();
        let mut live = db::open(&tmp.path().join("noma.db")).unwrap();
        add(&live, "before");
        let dir = tmp.path().join("backups");
        let snap = backup::snapshot(&live, &dir, Utc::now()).unwrap();
        add(&live, "after");
        assert_eq!(count(&live), 2);

        validate(&snap).unwrap();
        let name = snap.file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(resolve(&dir, &name).unwrap(), snap);
        assert!(resolve(&dir, "../noma.db").is_err());

        let staged = tmp.path().join("noma.db.restoring");
        fs::copy(&snap, &staged).unwrap();
        swap(&mut live, &staged).unwrap();
        assert_eq!(count(&live), 1);
        assert_eq!(list(&dir).len(), 1);
    }
}
