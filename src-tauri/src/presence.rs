//! Desktop presence extras: the menu-bar count, the orb's attention glow and
//! look (size, style), quiet hours, the orb's remembered position per screen
//! and the extra global shortcuts (Today, Search, last capture).
//!
//! A small loop recomputes the state every 30 s, on every item change and on
//! every UI setting change, and emits `presence:changed` only when it differs.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;

use chrono::Timelike;
use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Listener, Manager, Monitor, PhysicalPosition, State, WebviewWindow,
    WindowEvent,
};
use tokio::sync::Notify;

use crate::commands::ITEMS_CHANGED;
use crate::db::{
    self,
    settings::{get_setting, set_setting},
};
use crate::shortcuts::Action;
use crate::state::{AppState, local_tz, now};
use crate::windows;

pub const PRESENCE_CHANGED: &str = "presence:changed";
/// Main window: open the command palette.
pub const OPEN_PALETTE: &str = "ui:open-palette";
const TICK: StdDuration = StdDuration::from_secs(30);
const SAVE_AFTER: StdDuration = StdDuration::from_millis(500);
use crate::tray::TRAY_ID;

pub const MENUBAR_COUNT: &str = "ui.menubarCount";
pub const QUIET_ENABLED: &str = "reminders.quietEnabled";
pub const QUIET_FROM: &str = "reminders.quietFrom";
pub const QUIET_TO: &str = "reminders.quietTo";
const ORB_SIZE: &str = "orb.size";
const ORB_STYLE: &str = "orb.style";
const ORB_LAST_SCREEN: &str = "orb.lastScreen";

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PresenceState {
    /// A reminder is due within 15 minutes or overdue (never during quiet hours).
    pub attention: bool,
    /// Inside the daily quiet window: reminders wait, the orb stays still.
    pub quiet: bool,
    /// Open items on Today.
    pub today_left: u32,
    /// `s` · `m` · `l`.
    pub orb_size: String,
    /// `lively` · `calm` · `faded`.
    pub orb_style: String,
}

pub struct Presence {
    app: AppHandle,
    wake: Notify,
    last: Mutex<Option<PresenceState>>,
    moves: AtomicU64,
}

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

// ── Quiet hours ─────────────────────────────────────────────────────────

/// `HH:MM` → minutes after midnight.
pub fn parse_hm(value: &str) -> Option<u32> {
    let (h, m) = value.trim().split_once(':')?;
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// Whether `now` (minutes after midnight) falls in `[from, to)`, wrapping past
/// midnight when `to` is earlier than `from`. An empty window is never quiet.
pub fn in_window(from: u32, to: u32, now: u32) -> bool {
    match from.cmp(&to) {
        std::cmp::Ordering::Equal => false,
        std::cmp::Ordering::Less => from <= now && now < to,
        std::cmp::Ordering::Greater => now >= from || now < to,
    }
}

/// True inside the user's daily quiet window (Settings → On your desktop).
pub fn quiet_now(app: &AppHandle) -> bool {
    if setting(app, QUIET_ENABLED).as_deref() != Some("1") {
        return false;
    }
    let from = setting(app, QUIET_FROM)
        .as_deref()
        .and_then(parse_hm)
        .unwrap_or(22 * 60);
    let to = setting(app, QUIET_TO)
        .as_deref()
        .and_then(parse_hm)
        .unwrap_or(8 * 60);
    let local = now().with_timezone(&local_tz());
    in_window(from, to, local.hour() * 60 + local.minute())
}

// ── State loop ──────────────────────────────────────────────────────────

impl Presence {
    pub fn new(app: AppHandle) -> Arc<Self> {
        Arc::new(Self {
            app,
            wake: Notify::new(),
            last: Mutex::new(None),
            moves: AtomicU64::new(0),
        })
    }

    pub fn nudge(&self) {
        self.wake.notify_one();
    }

    pub fn current(&self) -> PresenceState {
        self.last
            .lock()
            .ok()
            .and_then(|l| l.clone())
            .unwrap_or_else(|| self.compute())
    }

    fn compute(&self) -> PresenceState {
        let (now, tz) = (now(), local_tz());
        let quiet = quiet_now(&self.app);
        let (today_left, due) = self
            .app
            .try_state::<AppState>()
            .and_then(|s| {
                let conn = s.db.lock().ok()?;
                Some((
                    db::desktop::today_left(&conn, now, tz).unwrap_or(0),
                    db::desktop::attention_count(&conn, now).unwrap_or(0),
                ))
            })
            .unwrap_or((0, 0));
        PresenceState {
            attention: due > 0 && !quiet,
            quiet,
            today_left,
            orb_size: setting(&self.app, ORB_SIZE).unwrap_or_else(|| "m".into()),
            orb_style: setting(&self.app, ORB_STYLE).unwrap_or_else(|| "lively".into()),
        }
    }

    fn refresh(&self) {
        let next = self.compute();
        self.update_tray(&next);
        let changed = match self.last.lock() {
            Ok(mut last) if last.as_ref() != Some(&next) => {
                *last = Some(next.clone());
                true
            }
            _ => false,
        };
        if changed {
            let _ = self.app.emit(PRESENCE_CHANGED, next);
        }
    }

    /// "3" next to the menu-bar symbol; nothing when 0 or switched off.
    fn update_tray(&self, state: &PresenceState) {
        let Some(tray) = self.app.tray_by_id(TRAY_ID) else {
            return;
        };
        let on = setting(&self.app, MENUBAR_COUNT).as_deref() != Some("0");
        let title = (on && state.today_left > 0).then(|| state.today_left.to_string());
        let _ = tray.set_title(title);
    }

    pub async fn run(self: Arc<Self>) {
        loop {
            self.refresh();
            tokio::select! {
                _ = self.wake.notified() => {}
                _ = tokio::time::sleep(TICK) => {}
            }
        }
    }
}

/// Starts the loop and the orb-position watcher.
pub fn setup(app: &AppHandle) {
    let presence = Presence::new(app.clone());
    app.manage(Arc::clone(&presence));
    let p = Arc::clone(&presence);
    app.listen_any(ITEMS_CHANGED, move |_| p.nudge());
    tauri::async_runtime::spawn(Arc::clone(&presence).run());

    if let Some(orb) = app.get_webview_window(windows::ORB) {
        let handle = app.clone();
        orb.on_window_event(move |event| {
            if let WindowEvent::Moved(_) = event {
                let Some(p) = handle.try_state::<Arc<Presence>>() else {
                    return;
                };
                let generation = p.moves.fetch_add(1, Ordering::SeqCst) + 1;
                let handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(SAVE_AFTER).await;
                    let Some(p) = handle.try_state::<Arc<Presence>>() else {
                        return;
                    };
                    if p.moves.load(Ordering::SeqCst) == generation {
                        save_orb_position(&handle);
                    }
                });
            }
        });
    }
}

/// Called by `settings_set` after any UI setting changed.
pub fn on_setting(app: &AppHandle, key: &str) {
    if key.starts_with("orb.") || key.starts_with("ui.") || key.starts_with("reminders.") {
        if let Some(p) = app.try_state::<Arc<Presence>>() {
            p.nudge();
        }
    }
}

// ── Orb position per screen ─────────────────────────────────────────────

fn screen_key(m: &Monitor) -> String {
    let name: String = m
        .name()
        .map(|n| {
            n.chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                .collect()
        })
        .unwrap_or_else(|| "screen".into());
    format!("{name}@{}x{}", m.size().width, m.size().height)
}

fn pos_key(screen: &str) -> String {
    format!("orb.pos.{screen}")
}

fn parse_xy(value: &str) -> Option<(f64, f64)> {
    let (x, y) = value.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

/// Keeps the orb itself on screen: the transparent margin of its window may
/// hang over the edge, the orb in the middle may not.
pub fn clamp_on_screen(
    (x, y): (i32, i32),
    (w, h): (i32, i32),
    (mx, my): (i32, i32),
    (mw, mh): (i32, i32),
) -> (i32, i32) {
    let (sx, sy) = (w * 35 / 100, h * 35 / 100);
    (
        x.clamp(mx - sx, (mx + mw - w + sx).max(mx - sx)),
        y.clamp(my - sy, (my + mh - h + sy).max(my - sy)),
    )
}

/// Remembers where the orb sits on the screen it is on (drag end).
fn save_orb_position(app: &AppHandle) {
    let Some(orb) = app.get_webview_window(windows::ORB) else {
        return;
    };
    let (Ok(pos), Ok(Some(m))) = (orb.outer_position(), orb.current_monitor()) else {
        return;
    };
    let scale = m.scale_factor();
    let lx = f64::from(pos.x - m.position().x) / scale;
    let ly = f64::from(pos.y - m.position().y) / scale;
    let key = screen_key(&m);
    store(app, &pos_key(&key), &format!("{lx:.0},{ly:.0}"));
    store(app, ORB_LAST_SCREEN, &key);
}

/// Puts the orb back where it was last dropped: on the last screen if it is
/// still connected, otherwise where it was on the current screen. Always
/// on-screen. Runs on the main thread (from `windows::show` / `toggle`).
pub fn restore_orb(app: &AppHandle, orb: &WebviewWindow) {
    let monitors = orb.available_monitors().unwrap_or_default();
    let current = orb
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| orb.primary_monitor().ok().flatten());
    let saved = |m: &Monitor| {
        setting(app, &pos_key(&screen_key(m)))
            .as_deref()
            .and_then(parse_xy)
    };
    let last = setting(app, ORB_LAST_SCREEN).and_then(|k| {
        monitors
            .iter()
            .find(|m| screen_key(m) == k)
            .and_then(|m| saved(m).map(|xy| (m.clone(), Some(xy))))
    });
    let Some((monitor, xy)) = last.or_else(|| current.map(|m| (m.clone(), saved(&m)))) else {
        return;
    };
    let (Ok(size), Ok(pos)) = (orb.outer_size(), orb.outer_position()) else {
        return;
    };
    let (mp, ms, scale) = (monitor.position(), monitor.size(), monitor.scale_factor());
    let target = match xy {
        Some((lx, ly)) => (
            mp.x + (lx * scale).round() as i32,
            mp.y + (ly * scale).round() as i32,
        ),
        None => (pos.x, pos.y),
    };
    let (x, y) = clamp_on_screen(
        target,
        (size.width as i32, size.height as i32),
        (mp.x, mp.y),
        (ms.width as i32, ms.height as i32),
    );
    if (x, y) != (pos.x, pos.y) {
        let _ = orb.set_position(PhysicalPosition::new(x, y));
    }
}

// ── Extra global shortcuts ──────────────────────────────────────────────

/// Today, Search and Open last capture: bring the main window forward.
pub fn run_shortcut(app: &AppHandle, action: Action) {
    let item = match action {
        Action::LastCapture => app.try_state::<AppState>().and_then(|s| {
            let conn = s.db.lock().ok()?;
            db::desktop::last_capture_item(&conn).ok().flatten()
        }),
        Action::Today | Action::Search => None,
        Action::Voice | Action::Capture => return,
    };
    windows::show_main(app);
    windows::hide(app, windows::PANEL);
    let _ = match action {
        Action::Today => app.emit(crate::filed::OPEN_TODAY, ()),
        Action::Search => app.emit(OPEN_PALETTE, ()),
        _ => match item {
            Some(id) => app.emit(crate::filed::OPEN_ITEM, id),
            None => Ok(()),
        },
    };
}

// ── Commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn presence_get(presence: State<'_, Arc<Presence>>) -> PresenceState {
    presence.current()
}

/// From the orb panel's search: the main window on that card.
#[tauri::command]
pub fn presence_open_item(app: AppHandle, item_id: String) {
    windows::show_main(&app);
    windows::hide(&app, windows::PANEL);
    let _ = app.emit(crate::filed::OPEN_ITEM, item_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_times() {
        assert_eq!(parse_hm("22:00"), Some(1320));
        assert_eq!(parse_hm("07:05"), Some(425));
        assert_eq!(parse_hm("24:00"), None);
        assert_eq!(parse_hm("nope"), None);
    }

    #[test]
    fn quiet_window_wraps_past_midnight() {
        let (from, to) = (22 * 60, 7 * 60);
        assert!(in_window(from, to, 23 * 60));
        assert!(in_window(from, to, 3 * 60));
        assert!(!in_window(from, to, 7 * 60));
        assert!(!in_window(from, to, 12 * 60));
        assert!(in_window(13 * 60, 14 * 60, 13 * 60 + 30));
        assert!(!in_window(13 * 60, 14 * 60, 14 * 60));
        assert!(!in_window(600, 600, 600));
    }

    #[test]
    fn orb_stays_on_screen() {
        let screen = ((0, 0), (1000, 800));
        let size = (100, 100);
        assert_eq!(
            clamp_on_screen((500, 400), size, screen.0, screen.1),
            (500, 400)
        );
        assert_eq!(
            clamp_on_screen((5000, -900), size, screen.0, screen.1),
            (935, -35)
        );
        // A second screen to the left (negative origin).
        assert_eq!(
            clamp_on_screen((0, 0), size, (-1440, 0), (1440, 900)),
            (-1440 + 1440 - 100 + 35, 0)
        );
    }

    #[test]
    fn xy_round_trip() {
        assert_eq!(parse_xy("12,-40"), Some((12.0, -40.0)));
        assert_eq!(parse_xy("x"), None);
    }
}
