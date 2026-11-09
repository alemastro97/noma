use std::collections::HashMap;

use tauri::{AppHandle, State};

use super::{emit_items_changed, lock_message, to_message};
use crate::db::{
    self,
    models::{Progress, Project},
};
use crate::state::{AppState, now};

#[tauri::command]
pub async fn projects_list(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::projects::list_projects(&conn).map_err(to_message)
}

#[tauri::command]
pub async fn projects_create(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    color: String,
) -> Result<Project, String> {
    let project = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::projects::create_project(&conn, &name, &color, now()).map_err(to_message)?
    };
    emit_items_changed(&app, vec![]);
    Ok(project)
}

#[tauri::command]
pub async fn projects_progress(
    state: State<'_, AppState>,
) -> Result<HashMap<String, Progress>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::items::project_progress(&conn).map_err(to_message)
}
