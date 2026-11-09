pub mod db;
mod windows;

use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// ⌥ Space. Phase 0 uses it to toggle the floating orb (spike); Phase 3 rebinds it
/// to the voice capture overlay.
fn orb_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::ALT), Code::Space)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());
    builder
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed && shortcut == &orb_shortcut() {
                        windows::orb::toggle(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            windows::orb::setup(app.handle());
            app.global_shortcut().register(orb_shortcut())?;
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.set_focus();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running NOMA");
}
