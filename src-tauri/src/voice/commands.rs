//! Tauri commands for the voice extras: kept recordings and undoing a voice
//! command from its confirmation toast.

use std::sync::Arc;

use tauri::{AppHandle, State, ipc::Response};

use super::{command, extras};
use crate::commands::{emit_items_changed, lock_message, to_message};
use crate::db::{self, voice::Recording};
use crate::reminders::Reminders;
use crate::state::{AppState, local_tz, now};

/// The kept audio behind a card, if any.
#[tauri::command]
pub async fn voice_recording_for_item(
    state: State<'_, AppState>,
    item_id: String,
) -> Result<Option<Recording>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::voice::recording_for_item(&conn, &item_id).map_err(to_message)
}

/// The WAV bytes of a recording (an ArrayBuffer on the JS side).
#[tauri::command]
pub async fn voice_recording_audio(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<Response, String> {
    let file = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::voice::get_recording(&conn, &id)
            .map_err(to_message)?
            .ok_or("recording not found")?
            .file
    };
    let dir = extras::recordings_dir(&app).ok_or("no app data directory")?;
    let bytes = std::fs::read(dir.join(file)).map_err(|e| e.to_string())?;
    Ok(Response::new(bytes))
}

/// Puts a card back the way it was before a voice command ("Undo" on its toast).
#[tauri::command]
pub async fn voice_command_undo(
    app: AppHandle,
    state: State<'_, AppState>,
    reminders: State<'_, Arc<Reminders>>,
    toast_id: String,
) -> Result<(), String> {
    let before = command::take_undo(&toast_id);
    reminders.dismiss(&toast_id);
    let Some(before) = before else {
        return Ok(());
    };
    {
        let conn = state.db.lock().map_err(lock_message)?;
        db::items::update_item(
            &conn,
            &before.id,
            command::restore(&before),
            now(),
            local_tz(),
        )
        .map_err(to_message)?;
    }
    emit_items_changed(&app, vec![before.id]);
    Ok(())
}
