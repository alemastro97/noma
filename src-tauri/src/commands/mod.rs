//! Tauri commands: thin adapters over `db::*`. Each mutation emits `items:changed`.

pub mod ai;
pub mod boards;
pub mod desktop;
pub mod items;
pub mod projects;
pub mod workspace;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::db::DbError;

pub const ITEMS_CHANGED: &str = "items:changed";

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemsChanged {
    pub ids: Vec<String>,
}

pub fn emit_items_changed(app: &AppHandle, ids: Vec<String>) {
    // Any change may move a reminder time.
    if let Some(r) = app.try_state::<std::sync::Arc<crate::reminders::Reminders>>() {
        r.nudge();
    }
    if let Err(err) = app.emit(ITEMS_CHANGED, ItemsChanged { ids }) {
        log::warn!("failed to emit {ITEMS_CHANGED}: {err}");
    }
}

/// Commands return `Result<T, String>`: Tauri serializes the error message for the frontend.
pub fn to_message(err: DbError) -> String {
    err.to_string()
}

pub fn lock_message<T>(_: std::sync::PoisonError<T>) -> String {
    "database lock poisoned".to_string()
}
