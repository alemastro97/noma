//! "Your data": restore from a snapshot, imports, folder auto-export, the
//! `noma://` URL scheme with its CLI, the MCP server, capture history and
//! personal stats. Everything stays on this Mac.

pub mod apple;
pub mod autoexport;
pub mod cli;
pub mod deeplink;
pub mod history;
pub mod import;
pub mod mcp;
pub mod restore;

use std::process::Command;

/// Runs a JavaScript for Automation snippet and returns what it printed.
pub(crate) fn jxa(script: &str) -> Result<String, String> {
    let out = Command::new("osascript")
        .args(["-l", "JavaScript", "-e", script])
        .output()
        .map_err(|e| format!("osascript: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// True when osascript failed because the user dismissed a dialog.
pub(crate) fn user_cancelled(err: &str) -> bool {
    err.contains("-128")
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpInfo {
    /// The `noma-mcp` binary next to NOMA's own executable.
    pub command: String,
    pub exists: bool,
    pub db_path: String,
}

/// Where the MCP server lives, for the setup snippet in Settings.
#[tauri::command]
pub async fn mcp_info(app: tauri::AppHandle) -> Result<McpInfo, String> {
    use tauri::Manager;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let command = exe.with_file_name("noma-mcp");
    let db_path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("noma.db");
    Ok(McpInfo {
        exists: command.is_file(),
        command: command.to_string_lossy().into_owned(),
        db_path: db_path.to_string_lossy().into_owned(),
    })
}
