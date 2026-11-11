//! Menu-bar item: a template N symbol with Capture, Quick capture, Open NOMA,
//! the floating-orb and pause-reminders switches, and Quit — labelled in the
//! interface language and relabelled live when it changes.

use std::sync::Arc;

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

use crate::db::settings::{get_setting, set_setting};
use crate::i18n;
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

/// Also used by presence.rs for the count next to the symbol.
pub(crate) const TRAY_ID: &str = "noma";

pub(crate) fn orb_enabled(app: &AppHandle) -> bool {
    setting(app, "orb.enabled").as_deref() != Some("0")
}

/// The menu in the current interface language (ids stay the same).
fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let lang = i18n::current(app);
    let voice = MenuItem::with_id(app, "voice", lang.tray_voice(), true, None::<&str>)?;
    let capture = MenuItem::with_id(app, "capture", lang.tray_capture(), true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", lang.tray_open(), true, None::<&str>)?;
    let orb = CheckMenuItem::with_id(
        app,
        "orb",
        lang.tray_orb(),
        true,
        orb_enabled(app),
        None::<&str>,
    )?;
    let paused = setting(app, "reminders.paused").as_deref() == Some("1");
    let pause =
        CheckMenuItem::with_id(app, "pause", lang.tray_pause(), true, paused, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", lang.tray_quit(), true, Some("Cmd+Q"))?;
    Menu::with_items(
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
    )
}

/// Rebuilds the menu after the interface language changed.
pub(crate) fn relabel(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    match build_menu(app) {
        Ok(menu) => {
            let _ = tray.set_menu(Some(menu));
        }
        Err(e) => log::warn!("tray menu: {e}"),
    }
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;

    let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))?;
    TrayIconBuilder::with_id(TRAY_ID)
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
