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

use crate::db::{self, models::Item, settings::get_setting};
use crate::state::AppState;
use crate::windows;

pub const REMINDERS_CHANGED: &str = "reminders:changed";
/// Older than this when it fires: it was missed, group it.
const MISSED_AFTER: Duration = Duration::minutes(10);
const MAX_SLEEP: StdDuration = StdDuration::from_secs(30);

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Toast {
    /// Item id, or "away" for the grouped toast.
    pub id: String,
    pub items: Vec<Item>,
    pub missed: bool,
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
                        },
                    ),
                }
            }
        }
        for item in &fresh {
            self.notify_native(&item.title, "Reminder from NOMA");
        }
        if !missed.is_empty() {
            let body = if missed.len() == 1 {
                missed[0].title.clone()
            } else {
                format!("{} reminders while you were away", missed.len())
            };
            self.notify_native("While you were away", &body);
        }
        self.publish();
        next
    }

    pub async fn run(self: Arc<Self>) {
        loop {
            let next = if self.paused() { None } else { self.tick() };
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
