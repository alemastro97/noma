pub mod ai;
mod backup;
pub mod commands;
pub mod data;
pub mod db;
mod filed;
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
        .plugin(tauri_plugin_deep_link::init())
        // Optional "open at login": starts hidden in the menu bar (`--background`).
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--background"]),
        ))
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
            tauri::async_runtime::spawn(backup::run(app.handle().clone()));
            data::autoexport::start(app.handle());
            data::deeplink::setup(app.handle());

            windows::setup(app.handle());
            if let Err(e) = tray::setup(app.handle()) {
                log::warn!("tray icon: {e}");
            }
            // Another app may already own a shortcut: NOMA still starts without it.
            let shortcuts = Shortcuts::load(app.handle());
            shortcuts.register_all(app.handle());
            app.manage(shortcuts);
            enable_login_item_once(app.handle());
            // Started at login: stay in the menu bar until the user opens NOMA.
            if std::env::args().any(|a| a == "--background") {
                windows::hide_main(app.handle());
                if tray::orb_enabled(app.handle()) {
                    windows::show(app.handle(), windows::ORB);
                }
            } else {
                windows::show_main(app.handle());
            }
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
            filed::filed_undo,
            filed::filed_move,
            filed::filed_open,
            filed::brief_open,
            backup::backup_info,
            backup::backup_now,
            backup::backup_reveal,
            backup::data_export,
            // data
            data::restore::backup_list,
            data::restore::backup_restore,
            data::import::import_text,
            data::import::import_apple_sources,
            data::import::import_apple,
            data::autoexport::pick_folder,
            data::autoexport::mirror_get,
            data::autoexport::mirror_set,
            data::cli::cli_info,
            data::cli::cli_install,
            data::history::captures_recent,
            data::history::captures_retry,
            data::history::stats_get,
            data::mcp_info,
            commands::desktop::autostart_get,
            commands::desktop::autostart_set,
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
            // ⌘Q closes the window but keeps NOMA (and ⌥ Space) alive; the menu-bar
            // "Quit NOMA" passes an exit code and really quits.
            tauri::RunEvent::ExitRequested { api, code, .. } if code.is_none() => {
                api.prevent_exit();
                windows::hide_main(app);
                if tray::orb_enabled(app) {
                    windows::show(app, windows::ORB);
                }
            }
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

/// First run of an installed build: start NOMA at login so the voice shortcut
/// works without opening the app. Runs once; the user's later choice in
/// Settings is never overridden. Skipped in dev builds, which would register
/// the throwaway dev binary.
fn enable_login_item_once(app: &tauri::AppHandle) {
    use tauri::Manager;
    use tauri_plugin_autostart::ManagerExt;
    const KEY: &str = "autostart.initialized";
    if cfg!(debug_assertions) {
        return;
    }
    let Some(state) = app.try_state::<state::AppState>() else {
        return;
    };
    let Ok(conn) = state.db.lock() else { return };
    if db::settings::get_setting(&conn, KEY)
        .ok()
        .flatten()
        .is_some()
    {
        return;
    }
    if let Err(e) = app.autolaunch().enable() {
        log::warn!("open at login: {e}");
    }
    let _ = db::settings::set_setting(&conn, KEY, "1");
}
