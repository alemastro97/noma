pub mod ai;
pub mod commands;
pub mod db;
pub mod state;
mod windows;

use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use std::sync::Arc;

use ai::service::AiService;
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
            let conn = db::open(&db_path).map_err(|e| {
                log::error!("could not open database at {}: {e}", db_path.display());
                e
            })?;
            app.manage(AppState::new(conn));

            let paths = ai::Paths::new(&app.path().app_data_dir()?);
            let service = AiService::new(app.handle().clone(), paths);
            app.manage(Arc::clone(&service));
            tauri::async_runtime::spawn(service.run_worker());

            windows::orb::setup(app.handle());
            // Another app may already own ⌥ Space: NOMA still starts, without the shortcut.
            if let Err(err) = app.global_shortcut().register(orb_shortcut()) {
                log::warn!("could not register ⌥ Space: {err}");
            }
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
            commands::items::items_duplicate,
            commands::items::items_counts,
            commands::items::items_search,
            commands::items::tags_list,
            commands::items::dev_seed,
            commands::projects::projects_list,
            commands::projects::projects_create,
            commands::projects::projects_progress,
            commands::boards::boards_list,
            commands::boards::boards_create,
            commands::boards::placements_list,
            commands::boards::placements_set,
            commands::boards::links_list,
            commands::boards::links_create,
            commands::boards::links_delete,
            commands::ai::ai_state,
            commands::ai::ai_refresh,
            commands::ai::ai_configure,
            commands::ai::ai_download,
            commands::ai::ai_cancel_download,
            commands::ai::ai_remove_model,
            commands::ai::capture_text,
            commands::ai::capture_retry,
        ])
        .build(tauri::generate_context!())
        .expect("error while building NOMA")
        .run(|app, event| {
            // Never leave a model server running after NOMA quits.
            if let tauri::RunEvent::Exit = event
                && let Some(ai) = app.try_state::<Arc<AiService>>()
            {
                ai.kill_server_now();
            }
        });
}
