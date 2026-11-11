//! Time features on the desktop: "before" reminders (offsets from the due
//! time), the evening recap, the focus timer, habits and custom snooze. Runs its
//! own small loop beside the reminder scheduler and shows its toasts through
//! [`Reminders::push`], using toast ids the toasts window recognizes:
//! `pre:<item id>`, `focus:<item id>` and `recap`.

use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Timelike, Utc};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::Notify;

use crate::commands::{emit_items_changed, lock_message, to_message};
use crate::db::{
    self,
    models::{Item, ItemFilter, ItemPatch, Status},
    settings::{get_setting, set_setting},
    timekeeping::{Habit, ItemTime},
};
use crate::reminders::{Reminders, Toast};
use crate::state::{AppState, local_tz, now};
use crate::windows;

pub const FOCUS_CHANGED: &str = "focus:changed";
pub const RECAP_ID: &str = "recap";
/// The evening recap shows from this local hour on.
const RECAP_FROM_HOUR: u32 = 19;
const RECAP_LAST_DAY: &str = "recap.last_day";
const DEFAULT_FOCUS_MINUTES: i64 = 25;
const MAX_SLEEP: StdDuration = StdDuration::from_secs(30);

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FocusState {
    pub item_id: String,
    pub title: String,
    pub started_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub minutes: i64,
}

pub struct TimeService {
    app: AppHandle,
    wake: Notify,
    focus: Mutex<Option<FocusState>>,
}

impl TimeService {
    pub fn new(app: AppHandle) -> Arc<Self> {
        Arc::new(Self {
            app,
            wake: Notify::new(),
            focus: Mutex::new(None),
        })
    }

    fn nudge(&self) {
        self.wake.notify_one();
    }

    fn focus(&self) -> Option<FocusState> {
        self.focus.lock().ok().and_then(|f| f.clone())
    }

    fn set_focus(&self, next: Option<FocusState>) {
        if let Ok(mut f) = self.focus.lock() {
            *f = next.clone();
        }
        let _ = self.app.emit(FOCUS_CHANGED, next);
        self.nudge();
    }

    fn setting(&self, key: &str) -> Option<String> {
        let state = self.app.try_state::<AppState>()?;
        let conn = state.db.lock().ok()?;
        get_setting(&conn, key).ok().flatten()
    }

    fn reminders(&self) -> Option<Arc<Reminders>> {
        self.app
            .try_state::<Arc<Reminders>>()
            .map(|r| Arc::clone(&r))
    }

    /// A native notification while the main window is not in front.
    fn notify_native(&self, title: &str, body: &str) {
        let in_front = self.reminders().is_some_and(|r| r.main_in_front());
        if in_front || self.setting("reminders.native").as_deref() == Some("0") {
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

    fn push(&self, toast: Toast) {
        if let Some(r) = self.reminders() {
            r.push(toast);
        }
    }

    /// Fires "before" reminders that are due; returns when the next one is.
    fn offsets(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let state = self.app.try_state::<AppState>()?;
        let (fired, next) = {
            let conn = state.db.lock().ok()?;
            let fired = db::timekeeping::take_due_offsets(&conn, now).unwrap_or_default();
            let next = db::timekeeping::next_offset_at(&conn, now).ok().flatten();
            (fired, next)
        };
        for (item, minutes) in fired {
            self.notify_native(&item.title, &before_label(minutes));
            self.push(Toast {
                id: format!("pre:{}", item.id),
                items: vec![item],
                missed: false,
                filed: None,
                brief: false,
            });
        }
        next
    }

    /// Once a day after 19:00: what is still open today and what got done.
    fn recap(&self, now: DateTime<Utc>) {
        if self.setting("reminders.recap").as_deref() == Some("0") {
            return;
        }
        let tz = local_tz();
        let local = now.with_timezone(&tz);
        let day = local.date_naive().to_string();
        if local.hour() < RECAP_FROM_HOUR || self.setting(RECAP_LAST_DAY).as_deref() == Some(&day) {
            return;
        }
        let Some(state) = self.app.try_state::<AppState>() else {
            return;
        };
        let items = {
            let Ok(conn) = state.db.lock() else { return };
            let _ = set_setting(&conn, RECAP_LAST_DAY, &day);
            db::items::list_items(&conn, &ItemFilter::Today { now, tz }, true).unwrap_or_default()
        };
        if items.is_empty() {
            return;
        }
        let done = items.iter().filter(|i| i.status == Status::Done).count();
        let open = items.len() - done;
        self.notify_native(
            "Your day in NOMA",
            &format!("{done} done, {open} still open"),
        );
        self.push(Toast {
            id: RECAP_ID.into(),
            items,
            missed: false,
            filed: None,
            brief: true,
        });
    }

    /// Ends the focus session when its time is up; returns its end otherwise.
    fn focus_tick(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let focus = self.focus()?;
        if focus.ends_at > now {
            return Some(focus.ends_at);
        }
        let item = self.app.try_state::<AppState>().and_then(|state| {
            let conn = state.db.lock().ok()?;
            let _ = db::timekeeping::log_focus(
                &conn,
                &focus.item_id,
                focus.started_at,
                focus.ends_at,
                true,
            );
            db::items::get_item(&conn, &focus.item_id).ok().flatten()
        });
        self.set_focus(None);
        if let Some(item) = item {
            self.notify_native("Focus session done", &item.title);
            self.push(Toast {
                id: format!("focus:{}", item.id),
                items: vec![item],
                missed: false,
                filed: None,
                brief: false,
            });
        }
        None
    }

    pub async fn run(self: Arc<Self>) {
        loop {
            let now = Utc::now();
            let paused = self.setting("reminders.paused").as_deref() == Some("1");
            let mut next = self.focus_tick(now);
            if !paused {
                self.recap(now);
                if let Some(at) = self.offsets(now) {
                    next = Some(next.map_or(at, |n| n.min(at)));
                }
            }
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

/// "1 hour before" style body for a "before" notification.
fn before_label(minutes: i64) -> String {
    let (n, unit) = if minutes % (60 * 24 * 7) == 0 {
        (minutes / (60 * 24 * 7), "week")
    } else if minutes % (60 * 24) == 0 {
        (minutes / (60 * 24), "day")
    } else if minutes % 60 == 0 {
        (minutes / 60, "hour")
    } else {
        (minutes, "minute")
    };
    let s = if n == 1 { "" } else { "s" };
    format!("Due in {n} {unit}{s}")
}

// ── Commands ─────────────────────────────────────────────────────────────

/// Snoozes a reminder until an exact moment (This evening, Next Monday, a picked time).
#[tauri::command]
pub async fn reminder_snooze_until(
    app: AppHandle,
    state: State<'_, AppState>,
    reminders: State<'_, Arc<Reminders>>,
    item_id: String,
    until: DateTime<Utc>,
) -> Result<(), String> {
    if until <= now() {
        return Err("pick a time in the future".into());
    }
    {
        let conn = state.db.lock().map_err(lock_message)?;
        let patch = ItemPatch {
            snoozed_until: Some(Some(until)),
            ..Default::default()
        };
        db::items::update_item(&conn, &item_id, patch, now(), local_tz()).map_err(to_message)?;
    }
    reminders.dismiss(&item_id);
    emit_items_changed(&app, vec![item_id]);
    Ok(())
}

/// Brings the main window forward on Today (from the evening recap).
#[tauri::command]
pub async fn recap_open(
    app: AppHandle,
    reminders: State<'_, Arc<Reminders>>,
) -> Result<(), String> {
    reminders.dismiss(RECAP_ID);
    windows::show_main(&app);
    let _ = app.emit(crate::filed::OPEN_TODAY, ());
    Ok(())
}

#[tauri::command]
pub async fn time_item_info(
    state: State<'_, AppState>,
    item_id: String,
) -> Result<ItemTime, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::timekeeping::item_time(&conn, &item_id, now(), local_tz()).map_err(to_message)
}

#[tauri::command]
pub async fn reminder_offsets_set(
    app: AppHandle,
    state: State<'_, AppState>,
    time: State<'_, Arc<TimeService>>,
    item_id: String,
    minutes: Vec<i64>,
) -> Result<Vec<i64>, String> {
    let offsets = {
        let conn = state.db.lock().map_err(lock_message)?;
        db::timekeeping::set_offsets(&conn, &item_id, &minutes).map_err(to_message)?
    };
    time.nudge();
    emit_items_changed(&app, vec![item_id]);
    Ok(offsets)
}

#[tauri::command]
pub async fn habit_set(
    app: AppHandle,
    state: State<'_, AppState>,
    item_id: String,
    on: bool,
) -> Result<(), String> {
    {
        let conn = state.db.lock().map_err(lock_message)?;
        db::timekeeping::set_habit(&conn, &item_id, on, now()).map_err(to_message)?;
    }
    emit_items_changed(&app, vec![item_id]);
    Ok(())
}

#[tauri::command]
pub async fn habits_list(state: State<'_, AppState>) -> Result<Vec<Habit>, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::timekeeping::list_habits(&conn, now(), local_tz()).map_err(to_message)
}

#[tauri::command]
pub async fn focus_current(
    time: State<'_, Arc<TimeService>>,
) -> Result<Option<FocusState>, String> {
    Ok(time.focus())
}

/// Starts a focus session on a card (25 minutes by default). A running one is
/// logged as stopped first.
#[tauri::command]
pub async fn focus_start(
    state: State<'_, AppState>,
    time: State<'_, Arc<TimeService>>,
    item_id: String,
    minutes: Option<i64>,
) -> Result<FocusState, String> {
    let minutes = minutes.unwrap_or(DEFAULT_FOCUS_MINUTES).clamp(1, 180);
    let started = now();
    let item: Item = {
        let conn = state.db.lock().map_err(lock_message)?;
        if let Some(old) = time.focus() {
            let _ = db::timekeeping::log_focus(&conn, &old.item_id, old.started_at, started, false);
        }
        db::items::get_item(&conn, &item_id)
            .map_err(to_message)?
            .ok_or("card not found")?
    };
    let focus = FocusState {
        item_id: item.id,
        title: item.title,
        started_at: started,
        ends_at: started + Duration::minutes(minutes),
        minutes,
    };
    if let Some(r) = time.reminders() {
        r.dismiss(&format!("focus:{item_id}"));
    }
    time.set_focus(Some(focus.clone()));
    Ok(focus)
}

/// Stops the running session early; the minutes so far are logged.
#[tauri::command]
pub async fn focus_stop(
    state: State<'_, AppState>,
    time: State<'_, Arc<TimeService>>,
) -> Result<(), String> {
    if let Some(old) = time.focus() {
        let conn = state.db.lock().map_err(lock_message)?;
        db::timekeeping::log_focus(&conn, &old.item_id, old.started_at, now(), false)
            .map_err(to_message)?;
    }
    time.set_focus(None);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::before_label;

    #[test]
    fn before_labels_use_the_largest_whole_unit() {
        assert_eq!(before_label(60), "Due in 1 hour");
        assert_eq!(before_label(24 * 60), "Due in 1 day");
        assert_eq!(before_label(15), "Due in 15 minutes");
        assert_eq!(before_label(2 * 24 * 60 * 7), "Due in 2 weeks");
    }
}
