//! Optional mirror of all cards as `noma.md` in a folder the user picks (an
//! Obsidian vault, iCloud Drive…), rewritten a few seconds after changes.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tauri::{AppHandle, Listener, Manager, State};
use tokio::sync::Notify;

use super::{jxa, user_cancelled};
use crate::backup;
use crate::commands::{ITEMS_CHANGED, lock_message, to_message};
use crate::db::settings::{get_setting, set_setting};
use crate::state::AppState;

pub const KEY_ENABLED: &str = "export.auto";
pub const KEY_FOLDER: &str = "export.folder";
pub const FILE_NAME: &str = "noma.md";
const DEBOUNCE: Duration = Duration::from_secs(5);

/// Listens for `items:changed` and rewrites the file once things settle.
pub fn start(app: &AppHandle) {
    let wake = Arc::new(Notify::new());
    let on_change = Arc::clone(&wake);
    app.listen_any(ITEMS_CHANGED, move |_| on_change.notify_one());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            wake.notified().await;
            // Debounce: wait until no change arrived for a while.
            while tokio::time::timeout(DEBOUNCE, wake.notified())
                .await
                .is_ok()
            {}
            if let Err(e) = export_now(&app) {
                log::warn!("auto-export failed: {e}");
            }
        }
    });
}

/// Writes `contents` atomically so a sync tool never sees half a file.
pub fn write_atomic(folder: &Path, contents: &str) -> std::io::Result<PathBuf> {
    fs::create_dir_all(folder)?;
    let target = folder.join(FILE_NAME);
    let tmp = folder.join(format!(".{FILE_NAME}.tmp"));
    fs::write(&tmp, contents)?;
    fs::rename(&tmp, &target)?;
    Ok(target)
}

/// Exports when enabled and a folder is set; returns the written path.
pub fn export_now(app: &AppHandle) -> Result<Option<PathBuf>, String> {
    let state = app.state::<AppState>();
    let (folder, md) = {
        let conn = state.db.lock().map_err(lock_message)?;
        let enabled = get_setting(&conn, KEY_ENABLED).map_err(to_message)?;
        let folder = get_setting(&conn, KEY_FOLDER).map_err(to_message)?;
        let (Some("1"), Some(folder)) = (enabled.as_deref(), folder) else {
            return Ok(None);
        };
        let export = backup::collect(&conn, Utc::now()).map_err(to_message)?;
        (folder, backup::markdown(&export))
    };
    write_atomic(Path::new(&folder), &md)
        .map(Some)
        .map_err(|e| e.to_string())
}

/// Native folder picker; `None` when the user cancels.
#[tauri::command]
pub async fn pick_folder(prompt: String) -> Result<Option<String>, String> {
    let prompt = serde_json::to_string(&prompt).map_err(|e| e.to_string())?;
    let script = format!(
        "const a = Application.currentApplication(); a.includeStandardAdditions = true;
         a.chooseFolder({{withPrompt: {prompt}}}).toString()"
    );
    tauri::async_runtime::spawn_blocking(move || match jxa(&script) {
        Ok(path) => Ok(Some(path)),
        Err(e) if user_cancelled(&e) => Ok(None),
        Err(e) => Err(e),
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MirrorConfig {
    pub enabled: bool,
    pub folder: Option<String>,
    /// Where the file was last written by this call, if it was.
    pub written: Option<String>,
}

fn config(state: &AppState) -> Result<MirrorConfig, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    Ok(MirrorConfig {
        enabled: get_setting(&conn, KEY_ENABLED)
            .map_err(to_message)?
            .as_deref()
            == Some("1"),
        folder: get_setting(&conn, KEY_FOLDER)
            .map_err(to_message)?
            .filter(|f| !f.is_empty()),
        written: None,
    })
}

#[tauri::command]
pub async fn mirror_get(state: State<'_, AppState>) -> Result<MirrorConfig, String> {
    config(&state)
}

/// Changes the switch and/or the folder, then writes the file right away.
#[tauri::command]
pub async fn mirror_set(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: Option<bool>,
    folder: Option<String>,
) -> Result<MirrorConfig, String> {
    {
        let conn = state.db.lock().map_err(lock_message)?;
        if let Some(on) = enabled {
            set_setting(&conn, KEY_ENABLED, if on { "1" } else { "0" }).map_err(to_message)?;
        }
        if let Some(folder) = folder {
            set_setting(&conn, KEY_FOLDER, &folder).map_err(to_message)?;
        }
    }
    let written = export_now(&app)?.map(|p| p.to_string_lossy().into_owned());
    Ok(MirrorConfig {
        written,
        ..config(&state)?
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_atomic_replaces_the_file() {
        let tmp = tempfile::tempdir().unwrap();
        let folder = tmp.path().join("vault");
        write_atomic(&folder, "one").unwrap();
        let path = write_atomic(&folder, "two").unwrap();
        assert_eq!(fs::read_to_string(path).unwrap(), "two");
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
    }
}
