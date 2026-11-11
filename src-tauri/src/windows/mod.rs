//! Desktop presence: the floating orb, its panel, the voice overlay, quick
//! capture and reminder toasts are small always-on-top panels declared in
//! `tauri.conf.json` (hidden at start) or, for the ones used rarely (overlay,
//! panel, toasts), created on first use: every webview costs 30–45 MB at rest.
//! All AppKit calls run on the main thread.

pub mod visibility;

#[cfg(target_os = "macos")]
mod macos;

use tauri::{
    AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

pub const MAIN: &str = "main";
pub const ORB: &str = "orb";
pub const OVERLAY: &str = "overlay";
pub const CAPTURE: &str = "capture";
pub const PANEL: &str = "panel";
pub const TOASTS: &str = "toasts";

pub const FLOATING: [&str; 5] = [ORB, OVERLAY, CAPTURE, PANEL, TOASTS];

/// (label, page, width, height, title) of the panels created on first use.
const LAZY: [(&str, &str, f64, f64, &str); 3] = [
    (
        OVERLAY,
        "src/windows/overlay/index.html",
        680.0,
        420.0,
        "NOMA voice",
    ),
    (
        PANEL,
        "src/windows/panel/index.html",
        340.0,
        540.0,
        "NOMA panel",
    ),
    (
        TOASTS,
        "src/windows/toasts/index.html",
        420.0,
        200.0,
        "NOMA reminders",
    ),
];

/// The window for `label`, created (hidden, as a panel) if it is lazy and not built yet.
/// Must run on the main thread.
fn ensure(app: &AppHandle, label: &str) -> Option<WebviewWindow> {
    if let Some(w) = app.get_webview_window(label) {
        return Some(w);
    }
    let (_, url, w, h, title) = LAZY.iter().find(|(l, ..)| *l == label)?;
    let built = WebviewWindowBuilder::new(app, label, WebviewUrl::App((*url).into()))
        .title(*title)
        .inner_size(*w, *h)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .skip_taskbar(true)
        .visible(false)
        .focused(false)
        .resizable(false)
        .accept_first_mouse(true)
        .build();
    match built {
        Ok(win) => {
            configure(&win);
            Some(win)
        }
        Err(e) => {
            log::warn!("could not create the {label} window: {e}");
            None
        }
    }
}

/// Panels that take keyboard input.
fn is_key(label: &str) -> bool {
    // The orb panel has a search and a capture field.
    matches!(label, OVERLAY | CAPTURE | PANEL)
}

fn configure(window: &WebviewWindow) {
    #[cfg(target_os = "macos")]
    macos::configure_floating(window, is_key(window.label()));
    #[cfg(not(target_os = "macos"))]
    let _ = window;
}

fn raw_show(window: &WebviewWindow) {
    #[cfg(target_os = "macos")]
    macos::show(window, is_key(window.label()));
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.show();
        if is_key(window.label()) {
            let _ = window.set_focus();
        }
    }
}

fn raw_hide(window: &WebviewWindow) {
    #[cfg(target_os = "macos")]
    macos::hide(window);
    #[cfg(not(target_os = "macos"))]
    let _ = window.hide();
}

fn raw_is_visible(window: &WebviewWindow) -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::is_visible(window)
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.is_visible().unwrap_or(false)
    }
}

/// Where a panel appears, relative to the screen it is on.
fn place(app: &AppHandle, window: &WebviewWindow) {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let (mx, my) = (monitor.position().x, monitor.position().y);
    let (mw, mh) = (monitor.size().width as i32, monitor.size().height as i32);
    let scale = monitor.scale_factor();
    let px = |logical: f64| (logical * scale).round() as i32;
    let Ok(size) = window.outer_size() else {
        return;
    };
    let (w, h) = (size.width as i32, size.height as i32);

    let pos = match window.label() {
        OVERLAY => Some((mx + (mw - w) / 2, my + mh * 18 / 100)),
        CAPTURE => Some((mx + (mw - w) / 2, my + mh * 26 / 100)),
        TOASTS => Some((mx + mw - w - px(12.0), my + px(34.0))),
        PANEL => app.get_webview_window(ORB).and_then(|orb| {
            let o = orb.outer_position().ok()?;
            let os = orb.outer_size().ok()?;
            let x = (o.x + os.width as i32 - w).clamp(mx, mx + mw - w);
            let above = o.y - h + px(24.0);
            let y = if above > my + px(30.0) {
                above
            } else {
                o.y + os.height as i32 - px(24.0)
            };
            Some((x, y))
        }),
        _ => None,
    };
    if let Some((x, y)) = pos {
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
}

/// Converts every floating window to a panel and puts the orb in its corner.
pub fn setup(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        for label in FLOATING {
            if let Some(w) = handle.get_webview_window(label) {
                configure(&w);
            }
        }
        if let Some(orb) = handle.get_webview_window(ORB)
            && let Ok(Some(m)) = orb.primary_monitor()
            && let Ok(size) = orb.outer_size()
        {
            let s = m.scale_factor();
            let x = m.position().x + m.size().width as i32 - size.width as i32 - (40.0 * s) as i32;
            let y =
                m.position().y + m.size().height as i32 - size.height as i32 - (60.0 * s) as i32;
            let _ = orb.set_position(PhysicalPosition::new(x, y));
        }
        if let Some(orb) = handle.get_webview_window(ORB) {
            crate::presence::restore_orb(&handle, &orb);
        }
    });
}

pub fn show(app: &AppHandle, label: &'static str) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = ensure(&handle, label) {
            configure(&w);
            if label != ORB {
                place(&handle, &w);
            } else if !raw_is_visible(&w) {
                crate::presence::restore_orb(&handle, &w);
            }
            raw_show(&w);
        }
    });
}

/// Whether a floating window exists and is on screen (blocks on the main thread).
pub fn is_shown(app: &AppHandle, label: &'static str) -> bool {
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let shown = handle
            .get_webview_window(label)
            .is_some_and(|w| raw_is_visible(&w));
        let _ = tx.send(shown);
    });
    rx.recv_timeout(std::time::Duration::from_secs(3))
        .unwrap_or(false)
}

pub fn hide(app: &AppHandle, label: &'static str) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = handle.get_webview_window(label) {
            raw_hide(&w);
        }
    });
}

pub fn toggle(app: &AppHandle, label: &'static str) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(w) = ensure(&handle, label) else {
            return;
        };
        if visibility::toggle_visibility(raw_is_visible(&w)) {
            configure(&w);
            if label != ORB {
                place(&handle, &w);
            } else {
                crate::presence::restore_orb(&handle, &w);
            }
            raw_show(&w);
        } else {
            raw_hide(&w);
        }
    });
}

/// Brings the main window back (and the Dock icon with it).
pub fn show_main(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(main) = app.get_webview_window(MAIN) {
        let _ = main.unminimize();
        let _ = main.show();
        let _ = main.set_focus();
    }
}

/// Closing the main window keeps NOMA running in the menu bar, without a Dock icon.
pub fn hide_main(app: &AppHandle) {
    if let Some(main) = app.get_webview_window(MAIN) {
        let _ = main.hide();
    }
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
}

/// Label from a frontend string, limited to the floating panels.
pub fn floating_label(label: &str) -> Option<&'static str> {
    FLOATING.iter().copied().find(|l| *l == label)
}
