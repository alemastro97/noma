//! Workspace commands: saved filters, canvas groups, the links graph, bulk
//! moves and delete snapshots for undo. Thin adapters over `db::workspace`.

use serde_json::Value;
use tauri::{AppHandle, State};

use super::{emit_items_changed, lock_message, to_message};
use crate::db::{
    self,
    filing::Place,
    models::Link,
    workspace::{CanvasGroup, GroupPatch, PriorPlace, SavedFilter, Snapshot},
};
use crate::state::{AppState, local_tz, now};

#[tauri::command]
pub async fn filters_list(state: State<'_, AppState>) -> Result<Vec<SavedFilter>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::workspace::list_filters(&conn).map_err(to_message)
}

#[tauri::command]
pub async fn filters_create(
    state: State<'_, AppState>,
    name: String,
    spec: Value,
) -> Result<SavedFilter, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::workspace::create_filter(&conn, &name, &spec, now()).map_err(to_message)
}

#[tauri::command]
pub async fn filters_delete(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::workspace::delete_filter(&conn, &id).map_err(to_message)
}

#[tauri::command]
pub async fn groups_list(
    state: State<'_, AppState>,
    board_id: String,
) -> Result<Vec<CanvasGroup>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::workspace::list_groups(&conn, &board_id).map_err(to_message)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn groups_create(
    state: State<'_, AppState>,
    board_id: String,
    label: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<CanvasGroup, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::workspace::create_group(&conn, &board_id, &label, x, y, w, h, now()).map_err(to_message)
}

/// Geometry changes come from a drag already shown on the canvas: no change event.
#[tauri::command]
pub async fn groups_update(
    state: State<'_, AppState>,
    id: String,
    patch: GroupPatch,
) -> Result<CanvasGroup, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::workspace::update_group(&conn, &id, patch).map_err(to_message)
}

#[tauri::command]
pub async fn groups_delete(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::workspace::delete_group(&conn, &id).map_err(to_message)
}

#[tauri::command]
pub async fn links_all(state: State<'_, AppState>) -> Result<Vec<Link>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::workspace::list_all_links(&conn).map_err(to_message)
}

/// Moves cards to a project, a personal board or the Inbox; returns where they were.
#[tauri::command]
pub async fn items_move(
    app: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<String>,
    place: Place,
) -> Result<Vec<PriorPlace>, String> {
    let prior = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::workspace::move_items(&conn, &ids, &place, now(), local_tz()).map_err(to_message)?
    };
    emit_items_changed(&app, ids);
    Ok(prior)
}

/// Undo of `items_move`.
#[tauri::command]
pub async fn items_unmove(
    app: AppHandle,
    state: State<'_, AppState>,
    prior: Vec<PriorPlace>,
) -> Result<(), String> {
    {
        let conn = state.db.lock().map_err(lock_message)?;
        db::workspace::restore_places(&conn, &prior, now(), local_tz()).map_err(to_message)?;
    }
    emit_items_changed(&app, prior.into_iter().map(|p| p.id).collect());
    Ok(())
}

#[tauri::command]
pub async fn items_snapshot(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<Vec<Snapshot>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::workspace::snapshot(&conn, &ids).map_err(to_message)
}

/// Brings back deleted cards from snapshots taken before the delete.
#[tauri::command]
pub async fn items_restore(
    app: AppHandle,
    state: State<'_, AppState>,
    snapshots: Vec<Snapshot>,
) -> Result<Vec<String>, String> {
    let ids = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::workspace::restore(&conn, &snapshots).map_err(to_message)?
    };
    emit_items_changed(&app, ids.clone());
    Ok(ids)
}
