//! AppKit specifics for floating windows. Only compiled on macOS.
//!
//! A regular `NSWindow` cannot float over *other* apps' fullscreen Spaces, no
//! matter its level or collection behavior. A non-activating `NSPanel` with
//! `FullScreenAuxiliary | CanJoinAllSpaces` can — this is what Spotlight-style
//! launchers do. `tauri-nspanel` swaps the window's class to an NSPanel subclass.

use objc2_app_kit::{NSWindowCollectionBehavior, NSWindowStyleMask};
use tauri::Manager;
use tauri_nspanel::{ManagerExt, PanelLevel, WebviewWindowExt, tauri_panel};

tauri_panel! {
    panel!(FloatingPanel {
        config: {
            can_become_key_window: false,
            is_floating_panel: true
        }
    })
}

fn floating_behavior() -> NSWindowCollectionBehavior {
    NSWindowCollectionBehavior::CanJoinAllSpaces
        | NSWindowCollectionBehavior::FullScreenAuxiliary
        | NSWindowCollectionBehavior::Stationary
        | NSWindowCollectionBehavior::IgnoresCycle
}

/// Converts `window` to a floating non-activating panel (once) and applies the
/// floating behavior. Idempotent: later calls only re-apply the behavior.
pub fn configure_floating(window: &tauri::WebviewWindow) {
    let app = window.app_handle();
    let label = window.label().to_string();

    let panel = match app.get_webview_panel(&label) {
        Ok(panel) => panel,
        Err(_) => match window.to_panel::<FloatingPanel>() {
            Ok(panel) => panel,
            Err(err) => {
                log::warn!("could not convert window {label} to panel: {err}");
                return;
            }
        },
    };

    let _ = panel.add_style_mask(NSWindowStyleMask::NonactivatingPanel);
    panel.set_collection_behavior(floating_behavior());
    // Status level sits above fullscreen apps' windows; +1 keeps us above menu-bar extras.
    panel.set_level(PanelLevel::Status.value() + 1);
    panel.set_hides_on_deactivate(false);
    panel.set_floating_panel(true);
}

/// Orders the panel front without activating the app. Falls back to the plain
/// window `show()` if the window was never converted.
pub fn order_front_regardless(window: &tauri::WebviewWindow) {
    match window.app_handle().get_webview_panel(window.label()) {
        Ok(panel) => panel.order_front_regardless(),
        Err(_) => {
            let _ = window.show();
        }
    }
}

/// Hides the panel (or the window if never converted).
pub fn order_out(window: &tauri::WebviewWindow) {
    match window.app_handle().get_webview_panel(window.label()) {
        Ok(panel) => panel.hide(),
        Err(_) => {
            let _ = window.hide();
        }
    }
}

/// Visibility that is correct for both panels and plain windows.
pub fn is_visible(window: &tauri::WebviewWindow) -> bool {
    match window.app_handle().get_webview_panel(window.label()) {
        Ok(panel) => panel.is_visible(),
        Err(_) => window.is_visible().unwrap_or(false),
    }
}
