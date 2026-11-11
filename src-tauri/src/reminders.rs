//! Reminder scheduler: wakes at the next reminder time (or every 30 s, which
//! also covers waking from sleep), shows a toast panel and, when NOMA is in the
//! background, a native notification. Reminders missed while the Mac slept or
//! NOMA was closed are grouped into one "while you were away" toast.

use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;

use chrono::{Duration, Utc};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::Notify;

use crate::db::{
    self,
    models::{Item, ItemFilter},
    settings::{get_setting, set_setting},
};
use crate::state::{AppState, local_tz};
use crate::windows;

pub const REMINDERS_CHANGED: &str = "reminders:changed";
/// Older than this when it fires: it was missed, group it.
const MISSED_AFTER: Duration = Duration::minutes(10);
const MAX_SLEEP: StdDuration = StdDuration::from_secs(30);
pub const BRIEF_ID: &str = "brief";
/// The morning brief shows from this local hour on.
const BRIEF_FROM_HOUR: u32 = 7;
const BRIEF_LAST_DAY: &str = "brief.last_day";

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Toast {
    /// Item id, or "away" for the grouped toast.
    pub id: String,
    pub items: Vec<Item>,
    pub missed: bool,
    /// Set on a "filed in…" toast shown after a capture made outside the main window.
    pub filed: Option<crate::filed::Filed>,
    /// The once-a-day "today" summary.
    pub brief: bool,
}

pub struct Reminders {
    app: AppHandle,
    wake: Notify,
    toasts: Mutex<Vec<Toast>>,
}

impl Reminders {
    pub fn new(app: AppHandle) -> Arc<Self> {
        Arc::new(Self {
            app,
            wake: Notify::new(),
            toasts: Mutex::new(Vec::new()),
        })
    }

    pub fn nudge(&self) {
        self.wake.notify_one();
    }

    pub fn active(&self) -> Vec<Toast> {
        self.toasts.lock().map(|t| t.clone()).unwrap_or_default()
    }

    fn publish(&self) {
        let toasts = self.active();
        if toasts.is_empty() {
            windows::hide(&self.app, windows::TOASTS);
        } else {
            windows::show(&self.app, windows::TOASTS);
        }
        let _ = self.app.emit(REMINDERS_CHANGED, toasts);
    }

    /// Adds a toast, replacing one with the same id.
    pub fn push(&self, toast: Toast) {
        if let Ok(mut t) = self.toasts.lock() {
            t.retain(|x| x.id != toast.id);
            t.push(toast);
        }
        self.publish();
    }

    /// Whether the main window is in front (the user already sees their cards).
    pub fn main_in_front(&self) -> bool {
        !self.app_in_background()
    }

    /// Removes the toast for `id` (an item id or "away").
    pub fn dismiss(&self, id: &str) {
        if let Ok(mut t) = self.toasts.lock() {
            t.retain(|x| x.id != id);
            for x in t.iter_mut() {
                x.items.retain(|i| i.id != id);
            }
            t.retain(|x| !x.items.is_empty());
        }
        self.publish();
    }

    fn paused(&self) -> bool {
        self.setting("reminders.paused").as_deref() == Some("1")
    }

    fn setting(&self, key: &str) -> Option<String> {
        let state = self.app.try_state::<AppState>()?;
        let conn = state.db.lock().ok()?;
        get_setting(&conn, key).ok().flatten()
    }

    fn app_in_background(&self) -> bool {
        self.app
            .get_webview_window(windows::MAIN)
            .and_then(|w| w.is_focused().ok())
            .map(|focused| !focused)
            .unwrap_or(true)
    }

    fn notify_native(&self, title: &str, body: &str) {
        if self.setting("reminders.native").as_deref() == Some("0") || !self.app_in_background() {
            return;
        }
        if let Err(e) = self
            .app
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
        {
            log::warn!("notification failed: {e}");
        }
    }

    /// Fires everything that is due now; returns when the next one is.
    fn tick(&self) -> Option<chrono::DateTime<Utc>> {
        let now = Utc::now();
        let state = self.app.try_state::<AppState>()?;
        let (due, next) = {
            let conn = state.db.lock().ok()?;
            let due = db::reminders::due_reminders(&conn, now).unwrap_or_default();
            for item in &due {
                let _ = db::reminders::mark_reminded(&conn, &item.id);
            }
            let next = db::reminders::next_reminder_at(&conn, now).ok().flatten();
            (due, next)
        };
        if due.is_empty() {
            return next;
        }

        let effective = |i: &Item| i.snoozed_until.or(i.remind_at).unwrap_or(now);
        let (missed, fresh): (Vec<Item>, Vec<Item>) = due
            .into_iter()
            .partition(|i| now - effective(i) > MISSED_AFTER);

        if let Ok(mut toasts) = self.toasts.lock() {
            for item in &fresh {
                toasts.retain(|t| t.id != item.id);
                toasts.push(Toast {
                    id: item.id.clone(),
                    items: vec![item.clone()],
                    missed: false,
                    filed: None,
                    brief: false,
                });
            }
            if !missed.is_empty() {
                match toasts.iter_mut().find(|t| t.id == "away") {
                    Some(away) => away.items.extend(missed.iter().cloned()),
                    None => toasts.insert(
                        0,
                        Toast {
                            id: "away".into(),
                            items: missed.clone(),
                            missed: true,
                            filed: None,
                            brief: false,
                        },
                    ),
                }
            }
        }
        let lang = crate::i18n::current(&self.app);
        for item in &fresh {
            self.notify_native(&item.title, lang.reminder_body());
        }
        if !missed.is_empty() {
            let body = if missed.len() == 1 {
                missed[0].title.clone()
            } else {
                lang.away_body(missed.len())
            };
            self.notify_native(lang.away_title(), &body);
        }
        self.publish();
        next
    }

    /// Once a day, the first time NOMA is awake after 7:00: what is on today.
    fn brief(&self) {
        use chrono::Timelike;
        if self.setting("reminders.brief").as_deref() == Some("0") {
            return;
        }
        let (now, tz) = (Utc::now(), local_tz());
        let local = now.with_timezone(&tz);
        let day = local.date_naive().to_string();
        if local.hour() < BRIEF_FROM_HOUR || self.setting(BRIEF_LAST_DAY).as_deref() == Some(&day) {
            return;
        }
        let Some(state) = self.app.try_state::<AppState>() else {
            return;
        };
        let items = {
            let Ok(conn) = state.db.lock() else { return };
            let _ = set_setting(&conn, BRIEF_LAST_DAY, &day);
            db::items::list_items(&conn, &ItemFilter::Today { now, tz }, false).unwrap_or_default()
        };
        if items.is_empty() {
            return;
        }
        let lang = crate::i18n::current(&self.app);
        let body = if items.len() == 1 {
            items[0].title.clone()
        } else {
            lang.brief_body(&items[0].title, items.len() - 1)
        };
        self.notify_native(lang.brief_title(), &body);
        self.push(Toast {
            id: BRIEF_ID.into(),
            items,
            missed: false,
            filed: None,
            brief: true,
        });
    }

    pub async fn run(self: Arc<Self>) {
        loop {
            // Quiet hours hold reminders like a pause; when they end, what fired
            // meanwhile is grouped into "While you were away".
            let hold = self.paused() || crate::presence::quiet_now(&self.app);
            if !hold {
                self.brief();
            }
            let next = if hold { None } else { self.tick() };
            let sleep = next
                .and_then(|n| (n - Utc::now()).to_std().ok())
                .map(|d| d.min(MAX_SLEEP))
                .unwrap_or(MAX_SLEEP);
            tokio::select! {
                _ = self.wake.notified() => {}
                _ = tokio::time::sleep(sleep) => {}
            }
        }
    }
}
