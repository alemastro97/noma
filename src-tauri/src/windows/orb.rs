use tauri::{AppHandle, Manager};

use super::visibility::toggle_visibility;

pub const ORB_WINDOW: &str = "orb";

/// Applies the floating configuration to the orb window (idempotent).
/// Must run on the main thread: AppKit silently ignores window changes from other threads.
pub fn setup(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window(ORB_WINDOW) {
            super::configure_floating(&window);
        }
    });
}

/// Shows or hides the orb window. Never creates a second window; the single
/// `orb` window is declared in `tauri.conf.json`. Safe to call from any thread
/// (the global-shortcut handler runs off the main thread).
pub fn toggle(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(window) = handle.get_webview_window(ORB_WINDOW) else {
            return;
        };
        if toggle_visibility(super::is_visible(&window)) {
            super::configure_floating(&window);
            let _ = super::show_without_focus(&window);
        } else {
            let _ = super::hide(&window);
        }
    });
}
