//! Card commands beyond editing: pin, archive, trash, subtasks, attachments,
//! and managing projects and personal boards. Thin adapters over `db::*`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tauri::{AppHandle, Manager, State};

use super::{emit_items_changed, lock_message, to_message};
use crate::db::{
    self,
    attachments::{Attachment, decode_base64, mime_for, safe_file_name},
    models::{Board, Item, Project, new_id},
    subtasks::Subtask,
};
use crate::state::{AppState, now};

/// Largest file accepted as an attachment.
const MAX_ATTACHMENT_BYTES: usize = 50 * 1024 * 1024;
const PURGE_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

fn attachments_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("attachments"))
}

fn open_path(path: &Path, reveal: bool) -> Result<(), String> {
    let mut cmd = std::process::Command::new("open");
    if reveal {
        cmd.arg("-R");
    }
    cmd.arg(path).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

/// Removes the attachment folders of cards that were deleted for good.
fn remove_attachment_dirs(app: &AppHandle, ids: &[String]) {
    let Ok(dir) = attachments_dir(app) else {
        return;
    };
    for id in ids {
        let _ = fs::remove_dir_all(dir.join(id));
    }
}

// ── Pin, archive, trash ─────────────────────────────────────────────────

#[tauri::command]
pub async fn items_pin(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    pinned: bool,
) -> Result<Item, String> {
    let item = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::cards::set_pinned(&conn, &id, pinned).map_err(to_message)?
    };
    emit_items_changed(&app, vec![id]);
    Ok(item)
}

#[tauri::command]
pub async fn items_archive(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    archived: bool,
) -> Result<Item, String> {
    let item = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::cards::set_archived(&conn, &id, archived, now()).map_err(to_message)?
    };
    emit_items_changed(&app, vec![id]);
    Ok(item)
}

#[tauri::command]
pub async fn items_trash(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<Item, String> {
    let item = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::cards::trash(&conn, &id, now()).map_err(to_message)?
    };
    emit_items_changed(&app, vec![id]);
    Ok(item)
}

#[tauri::command]
pub async fn items_restore(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<Item, String> {
    let item = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::cards::restore(&conn, &id).map_err(to_message)?
    };
    emit_items_changed(&app, vec![id]);
    Ok(item)
}

#[tauri::command]
pub async fn items_archived(state: State<'_, AppState>) -> Result<Vec<Item>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::cards::list_archived(&conn).map_err(to_message)
}

#[tauri::command]
pub async fn items_trashed(state: State<'_, AppState>) -> Result<Vec<Item>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::cards::list_trashed(&conn).map_err(to_message)
}

#[tauri::command]
pub async fn items_delete_forever(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let ids = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::cards::delete_forever(&conn, &id).map_err(to_message)?
    };
    remove_attachment_dirs(&app, &ids);
    emit_items_changed(&app, ids);
    Ok(())
}

#[tauri::command]
pub async fn trash_empty(app: AppHandle, state: State<'_, AppState>) -> Result<u32, String> {
    let ids = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::cards::empty_trash(&conn).map_err(to_message)?
    };
    remove_attachment_dirs(&app, &ids);
    let n = ids.len() as u32;
    emit_items_changed(&app, ids);
    Ok(n)
}

/// Purges old Trash and orphaned attachment folders now and every few hours.
pub async fn purge_loop(app: AppHandle) {
    loop {
        purge_once(&app);
        tokio::time::sleep(PURGE_EVERY).await;
    }
}

fn purge_once(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let (purged, live): (Vec<String>, Vec<String>) = {
        let Ok(conn) = state.db.lock() else { return };
        let purged = match db::cards::purge_trash(&conn, now()) {
            Ok(ids) => ids,
            Err(e) => {
                log::warn!("trash purge: {e}");
                return;
            }
        };
        let live = conn
            .prepare("SELECT DISTINCT item_id FROM attachments")
            .and_then(|mut s| {
                s.query_map([], |r| r.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .unwrap_or_default();
        (purged, live)
    };
    remove_attachment_dirs(app, &purged);
    // Folders left by cards deleted outside the Trash (undo of a capture, …).
    if let Ok(dir) = attachments_dir(app) {
        for entry in fs::read_dir(&dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.path().is_dir() && !live.contains(&name) {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
    if !purged.is_empty() {
        emit_items_changed(app, purged);
    }
}

// ── Subtasks ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn subtasks_list(
    state: State<'_, AppState>,
    item_id: String,
) -> Result<Vec<Subtask>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::subtasks::list_subtasks(&conn, &item_id).map_err(to_message)
}

#[tauri::command]
pub async fn subtasks_add(
    app: AppHandle,
    state: State<'_, AppState>,
    item_id: String,
    title: String,
) -> Result<Subtask, String> {
    let subtask = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::subtasks::add_subtask(&conn, &item_id, &title, now()).map_err(to_message)?
    };
    emit_items_changed(&app, vec![item_id]);
    Ok(subtask)
}

#[tauri::command]
pub async fn subtasks_update(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    title: Option<String>,
    done: Option<bool>,
) -> Result<Subtask, String> {
    let subtask = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::subtasks::update_subtask(&conn, &id, title.as_deref(), done).map_err(to_message)?
    };
    emit_items_changed(&app, vec![subtask.item_id.clone()]);
    Ok(subtask)
}

#[tauri::command]
pub async fn subtasks_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let item_id = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::subtasks::delete_subtask(&conn, &id).map_err(to_message)?
    };
    emit_items_changed(&app, vec![item_id]);
    Ok(())
}

#[tauri::command]
pub async fn subtasks_reorder(
    app: AppHandle,
    state: State<'_, AppState>,
    item_id: String,
    ids: Vec<String>,
) -> Result<(), String> {
    {
        let conn = state.db.lock().map_err(lock_message)?;
        db::subtasks::reorder_subtasks(&conn, &item_id, &ids).map_err(to_message)?;
    }
    emit_items_changed(&app, vec![item_id]);
    Ok(())
}

// ── Attachments ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn attachments_list(
    state: State<'_, AppState>,
    item_id: String,
) -> Result<Vec<Attachment>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::attachments::list_attachments(&conn, &item_id).map_err(to_message)
}

/// Copies a file (sent as base64) into the card's attachments folder.
#[tauri::command]
pub async fn attachments_add(
    app: AppHandle,
    state: State<'_, AppState>,
    item_id: String,
    name: String,
    data: String,
) -> Result<Attachment, String> {
    let bytes = decode_base64(&data).map_err(to_message)?;
    if bytes.len() > MAX_ATTACHMENT_BYTES {
        return Err(format!(
            "file is larger than {} MB",
            MAX_ATTACHMENT_BYTES / 1024 / 1024
        ));
    }
    {
        let conn = state.db.lock().map_err(lock_message)?;
        db::items::get_item(&conn, &item_id)
            .map_err(to_message)?
            .ok_or_else(|| format!("item {item_id} not found"))?;
    }
    let name = safe_file_name(&name);
    let id = new_id();
    let file = format!("{item_id}/{id}-{name}");
    let path = attachments_dir(&app)?.join(&file);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    let attachment = Attachment {
        id,
        item_id: item_id.clone(),
        mime: mime_for(&name).to_string(),
        name,
        file,
        size: bytes.len() as i64,
        created_at: now(),
    };
    let inserted = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::attachments::insert_attachment(&conn, &attachment)
    };
    if let Err(e) = inserted {
        let _ = fs::remove_file(&path);
        return Err(to_message(e));
    }
    emit_items_changed(&app, vec![item_id]);
    Ok(attachment)
}

fn attachment_path(
    app: &AppHandle,
    state: &State<'_, AppState>,
    id: &str,
) -> Result<PathBuf, String> {
    let a = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::attachments::get_attachment(&conn, id).map_err(to_message)?
    };
    Ok(attachments_dir(app)?.join(a.file))
}

/// The file's bytes (for image thumbnails).
#[tauri::command]
pub async fn attachments_read(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<tauri::ipc::Response, String> {
    let path = attachment_path(&app, &state, &id)?;
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    Ok(tauri::ipc::Response::new(bytes))
}

/// Opens the file in its default app (Preview for images and PDFs), or shows it in Finder.
#[tauri::command]
pub async fn attachments_open(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    reveal: Option<bool>,
) -> Result<(), String> {
    let path = attachment_path(&app, &state, &id)?;
    open_path(&path, reveal.unwrap_or(false))
}

#[tauri::command]
pub async fn attachments_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let a = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::attachments::delete_attachment(&conn, &id).map_err(to_message)?
    };
    let _ = fs::remove_file(attachments_dir(&app)?.join(&a.file));
    emit_items_changed(&app, vec![a.item_id]);
    Ok(())
}

/// Opens a link from a card's Markdown in the default browser (web and mail links only).
#[tauri::command]
pub async fn card_open_link(url: String) -> Result<(), String> {
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("https://")
        || lower.starts_with("http://")
        || lower.starts_with("mailto:"))
    {
        return Err("only web and mail links can be opened".into());
    }
    std::process::Command::new("open")
        .arg(&url)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Projects and boards ─────────────────────────────────────────────────

#[tauri::command]
pub async fn projects_rename(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> Result<Project, String> {
    let project = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::places::rename_project(&conn, &id, &name).map_err(to_message)?
    };
    emit_items_changed(&app, vec![]);
    Ok(project)
}

#[tauri::command]
pub async fn projects_set_color(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    color: String,
) -> Result<Project, String> {
    let project = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::places::set_project_color(&conn, &id, &color).map_err(to_message)?
    };
    emit_items_changed(&app, vec![]);
    Ok(project)
}

#[tauri::command]
pub async fn projects_archive(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    archived: bool,
) -> Result<Project, String> {
    let project = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::places::set_project_archived(&conn, &id, archived, now()).map_err(to_message)?
    };
    emit_items_changed(&app, vec![]);
    Ok(project)
}

#[tauri::command]
pub async fn projects_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<u32, String> {
    let n = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::places::delete_project(&conn, &id, now()).map_err(to_message)?
    };
    emit_items_changed(&app, vec![]);
    Ok(n as u32)
}

#[tauri::command]
pub async fn projects_archived(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::places::list_archived_projects(&conn).map_err(to_message)
}

#[tauri::command]
pub async fn boards_rename(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> Result<Board, String> {
    let board = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::places::rename_board(&conn, &id, &name).map_err(to_message)?
    };
    emit_items_changed(&app, vec![]);
    Ok(board)
}

#[tauri::command]
pub async fn boards_archive(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    archived: bool,
) -> Result<Board, String> {
    let board = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::places::set_board_archived(&conn, &id, archived, now()).map_err(to_message)?
    };
    emit_items_changed(&app, vec![]);
    Ok(board)
}

#[tauri::command]
pub async fn boards_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<u32, String> {
    let n = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::places::delete_board(&conn, &id, now()).map_err(to_message)?
    };
    emit_items_changed(&app, vec![]);
    Ok(n as u32)
}

#[tauri::command]
pub async fn boards_archived(state: State<'_, AppState>) -> Result<Vec<Board>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::places::list_archived_boards(&conn).map_err(to_message)
}
