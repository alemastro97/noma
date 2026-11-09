//! Menu-bar item: a template N symbol with Capture, Quick capture, Open NOMA,
//! the floating-orb and pause-reminders switches, and Quit.

use std::sync::Arc;

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

use crate::db::settings::{get_setting, set_setting};
use crate::reminders::Reminders;
use crate::state::AppState;
use crate::voice::session::VoiceService;
use crate::windows;

fn setting(app: &AppHandle, key: &str) -> Option<String> {
    let state = app.try_state::<AppState>()?;
    let conn = state.db.lock().ok()?;
    get_setting(&conn, key).ok().flatten()
}

fn store(app: &AppHandle, key: &str, value: &str) {
    if let Some(state) = app.try_state::<AppState>()
        && let Ok(conn) = state.db.lock()
    {
        let _ = set_setting(&conn, key, value);
    }
}

pub fn orb_enabled(app: &AppHandle) -> bool {
    setting(app, "orb.enabled").as_deref() != Some("0")
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let voice = MenuItem::with_id(app, "voice", "Capture by voice", true, Some("Alt+Space"))?;
    let capture = MenuItem::with_id(app, "capture", "Quick capture", true, Some("Cmd+Shift+N"))?;
    let open = MenuItem::with_id(app, "open", "Open NOMA", true, None::<&str>)?;
    let orb = CheckMenuItem::with_id(
        app,
        "orb",
        "Floating orb",
        true,
        orb_enabled(app),
        None::<&str>,
    )?;
    let paused = setting(app, "reminders.paused").as_deref() == Some("1");
    let pause =
        CheckMenuItem::with_id(app, "pause", "Pause reminders", true, paused, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit NOMA", true, Some("Cmd+Q"))?;
    let menu = Menu::with_items(
        app,
        &[
            &voice,
            &capture,
            &PredefinedMenuItem::separator(app)?,
            &open,
            &orb,
            &pause,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))?;
    TrayIconBuilder::with_id("noma")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("NOMA")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "voice" => {
                if let Some(v) = app.try_state::<Arc<VoiceService>>() {
                    let v = Arc::clone(&v);
                    tauri::async_runtime::spawn(async move { v.start().await });
                }
            }
            "capture" => windows::show(app, windows::CAPTURE),
            "open" => windows::show_main(app),
            "orb" => {
                let on = !orb_enabled(app);
                store(app, "orb.enabled", if on { "1" } else { "0" });
                if on {
                    windows::show(app, windows::ORB);
                } else {
                    windows::hide(app, windows::ORB);
                    windows::hide(app, windows::PANEL);
                }
            }
            "pause" => {
                let now_paused = setting(app, "reminders.paused").as_deref() != Some("1");
                store(app, "reminders.paused", if now_paused { "1" } else { "0" });
                if let Some(r) = app.try_state::<Arc<Reminders>>() {
                    r.nudge();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
