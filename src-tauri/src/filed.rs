//! "Filed in …" toast: after a capture made away from the main window (voice,
//! quick capture) NOMA says where the card went, with Undo, Move and Open.
//! Moving teaches the model: the correction becomes a filing example.

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::commands::{emit_items_changed, lock_message, to_message};
use crate::db::{
    self,
    filing::{self, Place},
    settings::get_setting,
};
use crate::reminders::{Reminders, Toast};
use crate::state::{AppState, local_tz, now};
use crate::windows;

pub const OPEN_ITEM: &str = "ui:open-item";
pub const OPEN_TODAY: &str = "ui:open-today";

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Filed {
    pub capture_id: String,
    pub place: Place,
    pub place_name: String,
    /// True once the user moved it: the toast confirms the new place.
    pub moved: bool,
}

fn toast_id(capture_id: &str) -> String {
    format!("filed:{capture_id}")
}

fn build(
    conn: &rusqlite::Connection,
    capture_id: &str,
    ids: &[String],
    moved: bool,
) -> Option<Toast> {
    let items: Vec<_> = ids
        .iter()
        .filter_map(|id| db::items::get_item(conn, id).ok().flatten())
        .collect();
    let first = items.first()?;
    let (place, place_name) = filing::place_of(conn, &first.id).ok()?;
    Some(Toast {
        id: toast_id(capture_id),
        items,
        missed: false,
        brief: false,
        filed: Some(Filed {
            capture_id: capture_id.to_string(),
            place,
            place_name,
            moved,
        }),
    })
}

/// Called by the AI worker when a capture became cards.
pub fn announce(app: &AppHandle, capture_id: &str, ids: &[String]) {
    let (Some(state), Some(reminders)) = (
        app.try_state::<AppState>(),
        app.try_state::<Arc<Reminders>>(),
    ) else {
        return;
    };
    // In the main window the card itself is the feedback.
    if reminders.main_in_front() {
        return;
    }
    let toast = {
        let Ok(conn) = state.db.lock() else { return };
        if get_setting(&conn, "capture.toast")
            .ok()
            .flatten()
            .as_deref()
            == Some("0")
        {
            return;
        }
        build(&conn, capture_id, ids, false)
    };
    if let Some(toast) = toast {
        reminders.push(toast);
    }
}

/// Deletes the cards a capture produced.
#[tauri::command]
pub async fn filed_undo(
    app: AppHandle,
    state: State<'_, AppState>,
    reminders: State<'_, Arc<Reminders>>,
    capture_id: String,
    item_ids: Vec<String>,
) -> Result<(), String> {
    {
        let conn = state.db.lock().map_err(lock_message)?;
        for id in &item_ids {
            // Already gone is fine.
            let _ = db::items::delete_item(&conn, id);
        }
    }
    reminders.dismiss(&toast_id(&capture_id));
    emit_items_changed(&app, item_ids);
    Ok(())
}

/// Moves a capture's cards to another place and remembers the correction.
#[tauri::command]
pub async fn filed_move(
    app: AppHandle,
    state: State<'_, AppState>,
    reminders: State<'_, Arc<Reminders>>,
    capture_id: String,
    item_ids: Vec<String>,
    place: Place,
) -> Result<(), String> {
    let toast = {
        let conn = state.db.lock().map_err(lock_message)?;
        for id in &item_ids {
            filing::move_item(&conn, id, &place, now(), local_tz()).map_err(to_message)?;
        }
        if let Some(c) = db::captures::get_capture(&conn, &capture_id).map_err(to_message)? {
            filing::record(&conn, &c.raw_text, &place, now()).map_err(to_message)?;
        }
        build(&conn, &capture_id, &item_ids, true)
    };
    match toast {
        Some(t) => reminders.push(t),
        None => reminders.dismiss(&toast_id(&capture_id)),
    }
    emit_items_changed(&app, item_ids);
    Ok(())
}

/// Brings the main window forward on a card.
#[tauri::command]
pub async fn filed_open(
    app: AppHandle,
    reminders: State<'_, Arc<Reminders>>,
    toast_id: String,
    item_id: String,
) -> Result<(), String> {
    reminders.dismiss(&toast_id);
    windows::show_main(&app);
    let _ = app.emit(OPEN_ITEM, item_id);
    Ok(())
}

/// Brings the main window forward on Today (from the morning brief).
#[tauri::command]
pub async fn brief_open(
    app: AppHandle,
    reminders: State<'_, Arc<Reminders>>,
) -> Result<(), String> {
    reminders.dismiss(crate::reminders::BRIEF_ID);
    windows::show_main(&app);
    let _ = app.emit(OPEN_TODAY, ());
    Ok(())
}
