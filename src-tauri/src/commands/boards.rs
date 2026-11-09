use tauri::{AppHandle, State};

use super::{emit_items_changed, lock_message, to_message};
use crate::db::{
    self,
    models::{Board, Placement},
};
use crate::state::{AppState, now};

#[tauri::command]
pub fn boards_list(state: State<'_, AppState>) -> Result<Vec<Board>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::boards::list_boards(&conn).map_err(to_message)
}

#[tauri::command]
pub fn boards_create(state: State<'_, AppState>, name: String) -> Result<Board, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::boards::create_board(&conn, &name, now()).map_err(to_message)
}

#[tauri::command]
pub fn placements_list(
    state: State<'_, AppState>,
    board_id: String,
) -> Result<Vec<Placement>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::placements::list_placements(&conn, &board_id).map_err(to_message)
}

#[tauri::command]
pub fn placements_set(
    app: AppHandle,
    state: State<'_, AppState>,
    item_id: String,
    board_id: String,
    x: f64,
    y: f64,
) -> Result<Placement, String> {
    let placement = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::placements::place_item(&conn, &item_id, &board_id, x, y).map_err(to_message)?
    };
    emit_items_changed(&app, vec![item_id]);
    Ok(placement)
}
