use tauri::{AppHandle, State};

use super::{emit_items_changed, lock_message, to_message};
use crate::db::{
    self,
    models::{Board, Link, Placement},
};
use crate::state::{AppState, now};

#[tauri::command]
pub async fn boards_list(state: State<'_, AppState>) -> Result<Vec<Board>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::boards::list_boards(&conn).map_err(to_message)
}

#[tauri::command]
pub async fn boards_create(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<Board, String> {
    let board = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::boards::create_board(&conn, &name, now()).map_err(to_message)?
    };
    emit_items_changed(&app, vec![]);
    Ok(board)
}

#[tauri::command]
pub async fn placements_list(
    state: State<'_, AppState>,
    board_id: String,
) -> Result<Vec<Placement>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::placements::list_placements(&conn, &board_id).map_err(to_message)
}

/// Moves a card on the canvas. Emits no change event: the canvas already shows
/// the new position, and a full reload on every drag would fight the gesture.
#[tauri::command]
pub async fn placements_set(
    state: State<'_, AppState>,
    item_id: String,
    board_id: String,
    x: f64,
    y: f64,
) -> Result<Placement, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::placements::place_item(&conn, &item_id, &board_id, x, y).map_err(to_message)
}

#[tauri::command]
pub async fn links_list(state: State<'_, AppState>, board_id: String) -> Result<Vec<Link>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::links::list_links_for_board(&conn, &board_id).map_err(to_message)
}

#[tauri::command]
pub async fn links_create(
    app: AppHandle,
    state: State<'_, AppState>,
    from_item: String,
    to_item: String,
) -> Result<Link, String> {
    let link = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::links::create_link(&conn, &from_item, &to_item, None, now()).map_err(to_message)?
    };
    emit_items_changed(&app, vec![link.from_item.clone(), link.to_item.clone()]);
    Ok(link)
}

#[tauri::command]
pub async fn links_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    {
        let conn = state.db.lock().map_err(lock_message)?;
        db::links::delete_link(&conn, &id).map_err(to_message)?;
    }
    emit_items_changed(&app, vec![]);
    Ok(())
}
