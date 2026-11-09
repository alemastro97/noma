use tauri::State;

use super::{lock_message, to_message};
use crate::db::{self, models::Project};
use crate::state::{AppState, now};

#[tauri::command]
pub fn projects_list(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::projects::list_projects(&conn).map_err(to_message)
}

#[tauri::command]
pub fn projects_create(
    state: State<'_, AppState>,
    name: String,
    color: String,
) -> Result<Project, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::projects::create_project(&conn, &name, &color, now()).map_err(to_message)
}
