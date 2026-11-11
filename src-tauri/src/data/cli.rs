//! A tiny `noma` shell command that talks to the app through `noma://`.

use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager};

pub const SCRIPT: &str = r#"#!/bin/sh
# noma — capture into NOMA from the terminal. Installed by NOMA (Settings → Your data).
#   noma buy milk tomorrow     capture text (or pipe it: echo "…" | noma)
#   noma open <card id>        show a card
#   noma today                 show Today
enc() { osascript -l JavaScript -e 'function run(a) { return encodeURIComponent(a[0]) }' "$1"; }
case "$1" in
  open)  [ -n "$2" ] || { echo "usage: noma open <card id>" >&2; exit 64; }
         exec open "noma://open?item=$(enc "$2")" ;;
  today) exec open "noma://today" ;;
  -h|--help|help) sed -n '2,6p' "$0"; exit 0 ;;
  *)     if [ $# -gt 0 ]; then text="$*"; else text="$(cat)"; fi
         [ -n "$text" ] || { echo "usage: noma <text>" >&2; exit 64; }
         exec open -g "noma://capture?text=$(enc "$text")" ;;
esac
"#;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliInfo {
    pub script: String,
    pub path: String,
    pub installed: bool,
}

fn target(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .home_dir()
        .map_err(|e| e.to_string())?
        .join(".local/bin/noma"))
}

#[tauri::command]
pub async fn cli_info(app: AppHandle) -> Result<CliInfo, String> {
    let path = target(&app)?;
    Ok(CliInfo {
        script: SCRIPT.to_string(),
        installed: path.is_file(),
        path: path.to_string_lossy().into_owned(),
    })
}

/// Writes the script to `~/.local/bin/noma` (executable).
#[tauri::command]
pub async fn cli_install(app: AppHandle) -> Result<CliInfo, String> {
    let path = target(&app)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(&path, SCRIPT).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    cli_info(app).await
}
