//! Tauri commands: thin adapters over `db::*`. Each mutation emits `items:changed`.

pub mod boards;
pub mod items;
pub mod projects;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::db::DbError;

pub const ITEMS_CHANGED: &str = "items:changed";

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemsChanged {
    pub ids: Vec<String>,
}

pub fn emit_items_changed(app: &AppHandle, ids: Vec<String>) {
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
