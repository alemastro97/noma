pub mod ai;
pub mod commands;
pub mod db;
pub mod reminders;
pub mod state;
mod tray;
pub mod voice;
mod windows;

use std::sync::Arc;

use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use ai::service::AiService;
use reminders::Reminders;
use state::AppState;
use voice::session::VoiceService;

/// ⌥ Space: voice capture (tap to toggle, hold to talk).
fn voice_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::ALT), Code::Space)
}

/// ⌘⇧N: quick capture.
fn capture_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyN)
}

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
                    if shortcut == &voice_shortcut() {
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
                    } else if shortcut == &capture_shortcut()
                        && event.state() == ShortcutState::Pressed
                    {
                        windows::toggle(app, windows::CAPTURE);
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
            let ai_service = AiService::new(app.handle().clone(), paths);
            app.manage(Arc::clone(&ai_service));
            tauri::async_runtime::spawn(ai_service.run_worker());

            app.manage(VoiceService::new(app.handle().clone()));

            let reminders = Reminders::new(app.handle().clone());
            app.manage(Arc::clone(&reminders));
            tauri::async_runtime::spawn(reminders.run());

            windows::setup(app.handle());
            if let Err(e) = tray::setup(app.handle()) {
                log::warn!("tray icon: {e}");
            }
            // Another app may already own a shortcut: NOMA still starts without it.
            for shortcut in [voice_shortcut(), capture_shortcut()] {
                if let Err(err) = app.global_shortcut().register(shortcut) {
                    log::warn!("could not register {shortcut:?}: {err}");
                }
            }
            windows::show_main(app.handle());
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
            }
            // Clicking the Dock icon brings the main window back.
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => windows::show_main(app),
            _ => {}
        });
}
