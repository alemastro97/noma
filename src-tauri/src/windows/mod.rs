pub mod orb;
pub mod visibility;

#[cfg(target_os = "macos")]
mod macos;

/// Makes `window` behave as a floating desktop element: above other windows,
/// present on every Space, shown over fullscreen apps, never stealing focus.
/// No-op on platforms other than macOS (their `always_on_top` config is enough).
pub fn configure_floating(window: &tauri::WebviewWindow) {
    #[cfg(target_os = "macos")]
    macos::configure_floating(window);
    #[cfg(not(target_os = "macos"))]
    let _ = window;
}

/// Shows `window` without activating the application (keeps the user's focus
/// where it was). Falls back to a plain `show()` where there is no native API.
pub fn show_without_focus(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        macos::order_front_regardless(window);
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.show()
    }
}

/// Hides `window` (panel-aware on macOS).
pub fn hide(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        macos::order_out(window);
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.hide()
    }
}

/// Current visibility (panel-aware on macOS).
pub fn is_visible(window: &tauri::WebviewWindow) -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::is_visible(window)
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.is_visible().unwrap_or(false)
    }
}
