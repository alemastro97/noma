//! AppKit specifics for floating windows. Only compiled on macOS.
//!
//! A regular `NSWindow` cannot float over *other* apps' fullscreen Spaces, no
//! matter its level or collection behavior. A non-activating `NSPanel` with
//! `FullScreenAuxiliary | CanJoinAllSpaces` can — this is what Spotlight-style
//! launchers do (ADR-0002). Panels that take typing (voice overlay, quick
//! capture) use a key-capable class.

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

    panel!(KeyPanel {
        config: {
            can_become_key_window: true,
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
/// floating behavior. `key` panels can take keyboard focus.
pub fn configure_floating(window: &tauri::WebviewWindow, key: bool) {
    let app = window.app_handle();
    let label = window.label().to_string();

    let panel = match app.get_webview_panel(&label) {
        Ok(panel) => panel,
        Err(_) => {
            let converted = if key {
                window.to_panel::<KeyPanel>()
            } else {
                window.to_panel::<FloatingPanel>()
            };
            match converted {
                Ok(panel) => panel,
                Err(err) => {
                    log::warn!("could not convert window {label} to panel: {err}");
                    return;
                }
            }
        }
    };

    let _ = panel.add_style_mask(NSWindowStyleMask::NonactivatingPanel);
    panel.set_collection_behavior(floating_behavior());
    // Status level sits above fullscreen apps' windows; +1 keeps us above menu-bar extras.
    panel.set_level(PanelLevel::Status.value() + 1);
    panel.set_hides_on_deactivate(false);
    panel.set_floating_panel(true);
}

/// Orders the panel front (making it key when it accepts typing) without
/// activating the app.
pub fn show(window: &tauri::WebviewWindow, key: bool) {
    match window.app_handle().get_webview_panel(window.label()) {
        Ok(panel) if key => panel.show_and_make_key(),
        Ok(panel) => panel.order_front_regardless(),
        Err(_) => {
            let _ = window.show();
        }
    }
}

/// Hides the panel (or the window if never converted).
pub fn hide(window: &tauri::WebviewWindow) {
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
