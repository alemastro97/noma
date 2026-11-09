use serde::Deserialize;
use tauri::{AppHandle, State};

use super::{emit_items_changed, lock_message, to_message};
use crate::db::{self, models::*};
use crate::state::{AppState, local_tz, now};

/// The filter as the frontend sends it. `now` and the time zone are injected here.
#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum WireFilter {
    Inbox,
    Today,
    Upcoming {
        #[serde(default = "default_days")]
        days: u32,
    },
    Ideas,
    Notes,
    Project {
        id: String,
    },
    Board {
        id: String,
    },
    All,
}

fn default_days() -> u32 {
    7
}

impl WireFilter {
    pub fn into_filter(self) -> ItemFilter {
        match self {
            WireFilter::Inbox => ItemFilter::Inbox,
            WireFilter::Today => ItemFilter::Today {
                now: now(),
                tz: local_tz(),
            },
            WireFilter::Upcoming { days } => ItemFilter::Upcoming { now: now(), days },
            WireFilter::Ideas => ItemFilter::Ideas,
            WireFilter::Notes => ItemFilter::Notes,
            WireFilter::Project { id } => ItemFilter::Project(id),
            WireFilter::Board { id } => ItemFilter::Board(id),
            WireFilter::All => ItemFilter::All,
        }
    }
}

#[tauri::command]
pub fn items_list(state: State<'_, AppState>, filter: WireFilter) -> Result<Vec<Item>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::items::list_items(&conn, &filter.into_filter()).map_err(to_message)
}

#[tauri::command]
pub fn items_get(state: State<'_, AppState>, id: String) -> Result<Option<Item>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::items::get_item(&conn, &id).map_err(to_message)
}

#[tauri::command]
pub fn items_create(
    app: AppHandle,
    state: State<'_, AppState>,
    input: NewItem,
) -> Result<Item, String> {
    let item = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::items::create_item(&conn, input, now()).map_err(to_message)?
    };
    emit_items_changed(&app, vec![item.id.clone()]);
    Ok(item)
}

#[tauri::command]
pub fn items_update(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    patch: ItemPatch,
) -> Result<Item, String> {
    let item = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::items::update_item(&conn, &id, patch, now()).map_err(to_message)?
    };
    emit_items_changed(&app, vec![item.id.clone()]);
    Ok(item)
}

#[tauri::command]
pub fn items_delete(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    {
        let conn = state.db.lock().map_err(lock_message)?;
        db::items::delete_item(&conn, &id).map_err(to_message)?;
    }
    emit_items_changed(&app, vec![id]);
    Ok(())
}

#[tauri::command]
pub fn items_complete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<Item, String> {
    let item = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::items::complete_item(&conn, &id, now()).map_err(to_message)?
    };
    emit_items_changed(&app, vec![item.id.clone()]);
    Ok(item)
}

#[tauri::command]
pub fn items_counts(state: State<'_, AppState>) -> Result<Counts, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::items::counts(&conn, now(), local_tz()).map_err(to_message)
}

#[tauri::command]
pub fn items_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<Item>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::items::search_items(&conn, &query, limit.unwrap_or(20)).map_err(to_message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_filter_deserializes_tagged_by_kind() {
        let f: WireFilter = serde_json::from_str(r#"{"kind":"project","id":"p1"}"#).unwrap();
        assert!(matches!(f.into_filter(), ItemFilter::Project(id) if id == "p1"));
        let f: WireFilter = serde_json::from_str(r#"{"kind":"upcoming"}"#).unwrap();
        assert!(matches!(
            f.into_filter(),
            ItemFilter::Upcoming { days: 7, .. }
        ));
        let f: WireFilter = serde_json::from_str(r#"{"kind":"today"}"#).unwrap();
        assert!(matches!(f.into_filter(), ItemFilter::Today { .. }));
    }
}
