//! Local safety net: a daily snapshot of the database (the newest ones are
//! kept) and a readable export. Everything stays on this Mac.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Local, Utc};
use rusqlite::Connection;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::commands::{lock_message, to_message};
use crate::db::{
    self, DbResult, filing,
    models::{Item, ItemFilter, Status},
};
use crate::state::AppState;

const KEEP: usize = 10;
const EVERY: chrono::Duration = chrono::Duration::hours(24);
const CHECK_EVERY: Duration = Duration::from_secs(60 * 60);

pub fn dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("backups"))
}

/// Snapshot files, newest first.
pub(crate) fn snapshots(dir: &Path) -> Vec<(PathBuf, DateTime<Utc>)> {
    let mut all: Vec<_> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name.starts_with("noma-") && name.ends_with(".db")
        })
        .filter_map(|e| {
            let modified = e.metadata().ok()?.modified().ok()?;
            Some((e.path(), DateTime::<Utc>::from(modified)))
        })
        .collect();
    all.sort_by_key(|a| std::cmp::Reverse(a.1));
    all
}

/// Writes a consistent copy of the live database and prunes old snapshots.
pub fn snapshot(conn: &Connection, dir: &Path, now: DateTime<Utc>) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let name = format!(
        "noma-{}.db",
        now.with_timezone(&Local).format("%Y-%m-%d-%H%M%S")
    );
    let path = dir.join(name);
    let _ = fs::remove_file(&path);
    conn.execute("VACUUM INTO ?1", [path.to_string_lossy().as_ref()])
        .map_err(|e| e.to_string())?;
    for (old, _) in snapshots(dir).into_iter().skip(KEEP) {
        let _ = fs::remove_file(old);
    }
    Ok(path)
}

/// Takes a snapshot when the newest one is older than a day. Runs for the app's lifetime.
pub async fn run(app: AppHandle) {
    loop {
        if let (Ok(dir), Some(state)) = (dir(&app), app.try_state::<AppState>()) {
            let due = snapshots(&dir)
                .first()
                .is_none_or(|(_, at)| Utc::now() - *at > EVERY);
            if due
                && let Ok(conn) = state.db.lock()
                && let Err(e) = snapshot(&conn, &dir, Utc::now())
            {
                log::warn!("backup failed: {e}");
            }
        }
        tokio::time::sleep(CHECK_EVERY).await;
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub dir: String,
    pub count: usize,
    pub last_at: Option<DateTime<Utc>>,
}

fn info(dir: &Path) -> BackupInfo {
    let all = snapshots(dir);
    BackupInfo {
        dir: dir.to_string_lossy().into_owned(),
        count: all.len(),
        last_at: all.first().map(|(_, at)| *at),
    }
}

#[tauri::command]
pub async fn backup_info(app: AppHandle) -> Result<BackupInfo, String> {
    Ok(info(&dir(&app)?))
}

#[tauri::command]
pub async fn backup_now(app: AppHandle, state: State<'_, AppState>) -> Result<BackupInfo, String> {
    let dir = dir(&app)?;
    {
        let conn = state.db.lock().map_err(lock_message)?;
        snapshot(&conn, &dir, Utc::now())?;
    }
    Ok(info(&dir))
}

pub(crate) fn reveal(path: &Path, select: bool) {
    let mut cmd = std::process::Command::new("open");
    if select {
        cmd.arg("-R");
    }
    if let Err(e) = cmd.arg(path).spawn() {
        log::warn!("could not open {}: {e}", path.display());
    }
}

#[tauri::command]
pub async fn backup_reveal(app: AppHandle) -> Result<(), String> {
    let dir = dir(&app)?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    reveal(&dir, false);
    Ok(())
}

// ── Export ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportedItem {
    #[serde(flatten)]
    item: Item,
    place: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Export {
    exported_at: DateTime<Utc>,
    projects: Vec<db::models::Project>,
    boards: Vec<db::models::Board>,
    items: Vec<ExportedItem>,
}

pub(crate) fn collect(conn: &Connection, now: DateTime<Utc>) -> DbResult<Export> {
    let items = db::items::list_items(conn, &ItemFilter::All, true)?
        .into_iter()
        .map(|item| {
            let place = filing::place_of(conn, &item.id)
                .map(|(_, name)| name)
                .unwrap_or_else(|_| filing::INBOX_NAME.to_string());
            ExportedItem { item, place }
        })
        .collect();
    Ok(Export {
        exported_at: now,
        projects: db::projects::list_projects(conn)?,
        boards: db::boards::list_boards(conn)?,
        items,
    })
}

/// Everything as Markdown: one section per place, open cards first.
pub(crate) fn markdown(export: &Export) -> String {
    let mut out = format!(
        "# NOMA export\n\n{}\n",
        export
            .exported_at
            .with_timezone(&Local)
            .format("%Y-%m-%d %H:%M")
    );
    let mut places: Vec<&str> = vec![filing::INBOX_NAME];
    for item in &export.items {
        if !places.contains(&item.place.as_str()) {
            places.push(&item.place);
        }
    }
    for place in places {
        let cards: Vec<&ExportedItem> = export.items.iter().filter(|i| i.place == place).collect();
        if cards.is_empty() {
            continue;
        }
        out.push_str(&format!("\n## {place}\n\n"));
        for card in cards {
            let i = &card.item;
            let check = if i.status == Status::Done { "x" } else { " " };
            let when = i
                .due_at
                .or(i.remind_at)
                .map(|d| format!(" — {}", d.with_timezone(&Local).format("%Y-%m-%d %H:%M")))
                .unwrap_or_default();
            out.push_str(&format!("- [{check}] {}{when}\n", i.title));
            if let Some(body) = &i.body {
                for line in body.lines().filter(|l| !l.trim().is_empty()) {
                    out.push_str(&format!("  > {line}\n"));
                }
            }
        }
    }
    out
}

/// Writes `noma.json` and `noma.md` to a dated folder in Downloads and shows it.
#[tauri::command]
pub async fn data_export(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let export = {
        let conn = state.db.lock().map_err(lock_message)?;
        collect(&conn, Utc::now()).map_err(to_message)?
    };
    let base = app
        .path()
        .download_dir()
        .or_else(|_| app.path().home_dir())
        .map_err(|e| e.to_string())?;
    let folder = base.join(format!(
        "NOMA export {}",
        Local::now().format("%Y-%m-%d %H.%M.%S")
    ));
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&export).map_err(|e| e.to_string())?;
    fs::write(folder.join("noma.json"), json).map_err(|e| e.to_string())?;
    fs::write(folder.join("noma.md"), markdown(&export)).map_err(|e| e.to_string())?;
    reveal(&folder, true);
    Ok(folder.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::filing::Place;
    use crate::db::models::NewItem;

    #[test]
    fn snapshot_is_a_readable_copy_and_old_ones_are_pruned() {
        let tmp = tempfile::tempdir().unwrap();
        let conn = db::open(&tmp.path().join("live.db")).unwrap();
        db::items::create_item(
            &conn,
            NewItem {
                title: "keep me".into(),
                ..Default::default()
            },
            Utc::now(),
        )
        .unwrap();
        let dir = tmp.path().join("backups");
        let mut last = PathBuf::new();
        for n in 0..(KEEP as i64 + 3) {
            last = snapshot(&conn, &dir, Utc::now() + chrono::Duration::seconds(n)).unwrap();
        }
        assert!(snapshots(&dir).len() <= KEEP);
        let copy = Connection::open(&last).unwrap();
        let n: u32 = copy
            .query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn markdown_groups_cards_by_place() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let project = db::projects::create_project(&conn, "Studio move", "#fff", now).unwrap();
        let a = db::items::create_item(
            &conn,
            NewItem {
                title: "Call movers".into(),
                body: Some("ask for a quote".into()),
                ..Default::default()
            },
            now,
        )
        .unwrap();
        filing::move_item(
            &conn,
            &a.id,
            &Place::Project(project.id),
            now,
            chrono_tz::UTC,
        )
        .unwrap();
        db::items::create_item(
            &conn,
            NewItem {
                title: "Loose thought".into(),
                ..Default::default()
            },
            now,
        )
        .unwrap();
        let md = markdown(&collect(&conn, now).unwrap());
        let inbox = md.find("## Inbox").unwrap();
        let studio = md.find("## Studio move").unwrap();
        assert!(inbox < studio);
        assert!(md.contains("- [ ] Call movers\n  > ask for a quote"));
        assert!(md.contains("- [ ] Loose thought"));
    }
}
