pub mod commands;
pub mod db;
pub mod state;
mod windows;

use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use state::AppState;

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
            let db_path = app.path().app_data_dir()?.join("noma.db");
            let conn = db::open(&db_path)?;
            app.manage(AppState::new(conn));

            windows::orb::setup(app.handle());
            app.global_shortcut().register(orb_shortcut())?;
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.set_focus();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::items::items_list,
            commands::items::items_get,
            commands::items::items_create,
            commands::items::items_update,
            commands::items::items_delete,
            commands::items::items_complete,
            commands::items::items_counts,
            commands::items::items_search,
            commands::projects::projects_list,
            commands::projects::projects_create,
            commands::boards::boards_list,
            commands::boards::boards_create,
            commands::boards::placements_list,
            commands::boards::placements_set,
        ])
        .run(tauri::generate_context!())
        .expect("error while running NOMA");
}
