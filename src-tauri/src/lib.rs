pub mod ai;
pub mod commands;
pub mod db;
pub mod reminders;
#[cfg(debug_assertions)]
mod selftest;
pub mod shortcuts;
pub mod state;
mod tray;
pub mod voice;
mod windows;

use std::sync::Arc;

use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::ShortcutState;

use ai::service::AiService;
use reminders::Reminders;
use shortcuts::{Action, Shortcuts};
use state::AppState;
use voice::session::VoiceService;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());
    builder
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    let Some(action) = app
                        .try_state::<Shortcuts>()
                        .and_then(|s| s.action_for(shortcut))
                    else {
                        return;
                    };
                    if action == Action::Voice {
                        let Some(voice) = app.try_state::<Arc<VoiceService>>() else {
                            return;
                        };
                        let voice = Arc::clone(&voice);
                        let pressed = event.state() == ShortcutState::Pressed;
                        tauri::async_runtime::spawn(async move {
                            if pressed {
                                voice.on_press().await;
                            } else {
                                voice.on_release().await;
                            }
                        });
                    } else if event.state() == ShortcutState::Pressed {
                        windows::toggle(app, windows::CAPTURE);
                    }
                })
                .build(),
        )
        .setup(|app| {
            // Debug self-test runs on a throwaway database.
            #[cfg(debug_assertions)]
            let db_path = std::env::var_os("NOMA_SELFTEST_DB")
                .map(std::path::PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?.join("noma.db"));
            #[cfg(not(debug_assertions))]
            let db_path = app.path().app_data_dir()?.join("noma.db");
            let conn = db::open(&db_path).map_err(|e| {
                log::error!("could not open database at {}: {e}", db_path.display());
                e
            })?;
            app.manage(AppState::new(conn));

            let paths = ai::Paths::new(&app.path().app_data_dir()?);
            let ai_service = AiService::new(app.handle().clone(), paths);
            app.manage(Arc::clone(&ai_service));
            tauri::async_runtime::spawn(ai_service.run_worker());

            let voice = VoiceService::new(app.handle().clone());
            app.manage(Arc::clone(&voice));

            let reminders = Reminders::new(app.handle().clone());
            app.manage(Arc::clone(&reminders));
            tauri::async_runtime::spawn(reminders.run());

            windows::setup(app.handle());
            if let Err(e) = tray::setup(app.handle()) {
                log::warn!("tray icon: {e}");
            }
            // Another app may already own a shortcut: NOMA still starts without it.
            let shortcuts = Shortcuts::load(app.handle());
            shortcuts.register_all(app.handle());
            app.manage(shortcuts);
            windows::show_main(app.handle());
            #[cfg(debug_assertions)]
            if std::env::var_os("NOMA_SELFTEST_DB").is_some() {
                tauri::async_runtime::spawn(selftest::run(app.handle().clone()));
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != windows::MAIN {
                return;
            }
            let app = window.app_handle();
            match event {
                // Closing the window keeps NOMA in the menu bar.
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    windows::hide_main(app);
                    if tray::orb_enabled(app) {
                        windows::show(app, windows::ORB);
                    }
                }
                // The orb steps aside while the main window is in front.
                WindowEvent::Focused(true) => {
                    windows::hide(app, windows::ORB);
                    windows::hide(app, windows::PANEL);
                }
                WindowEvent::Focused(false) if tray::orb_enabled(app) => {
                    windows::show(app, windows::ORB);
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::desktop::shortcuts_get,
            commands::desktop::shortcuts_set,
            commands::desktop::shortcuts_reset,
            commands::desktop::shortcuts_suspend,
            commands::desktop::shortcuts_resume,
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
            commands::desktop::voice_start,
            commands::desktop::voice_stop,
            commands::desktop::voice_cancel,
            commands::desktop::voice_edit,
            commands::desktop::voice_current,
            commands::desktop::panel_show,
            commands::desktop::panel_hide,
            commands::desktop::panel_toggle,
            commands::desktop::main_show,
            commands::desktop::reminders_active,
            commands::desktop::reminder_dismiss,
            commands::desktop::reminder_snooze,
            commands::desktop::reminder_done,
            commands::desktop::settings_get,
            commands::desktop::settings_set,
        ])
        .build(tauri::generate_context!())
        .expect("error while building NOMA")
        .run(|app, event| match event {
            // Never leave a model server running after NOMA quits.
            tauri::RunEvent::Exit => {
                if let Some(ai) = app.try_state::<Arc<AiService>>() {
                    ai.kill_server_now();
                }
                // Whisper's Metal device asserts at process exit if a model is still
                // loaded, which macOS reports as a crash: free it first.
                if let Some(voice) = app.try_state::<Arc<VoiceService>>() {
                    voice.shutdown();
                }
            }
            // Clicking the Dock icon brings the main window back.
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => windows::show_main(app),
            _ => {}
        });
}
