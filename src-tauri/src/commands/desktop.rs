//! Commands for the desktop presence: voice, floating panels, reminders and
//! the small UI settings the panels share.

use std::sync::Arc;

use chrono::{Duration, TimeZone, Utc};
use tauri::{AppHandle, State};

use super::{emit_items_changed, lock_message, to_message};
use crate::db::{
    self,
    models::ItemPatch,
    settings::{get_setting, set_setting},
};
use crate::reminders::{Reminders, Toast};
use crate::state::{AppState, local_tz, now};
use crate::voice::session::{Finish, VoiceService, VoiceState};
use crate::windows;

// ── Voice ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn voice_start(voice: State<'_, Arc<VoiceService>>) -> Result<(), String> {
    voice.start().await;
    Ok(())
}

/// Stop listening and save the capture now (↵ in the overlay).
#[tauri::command]
pub async fn voice_stop(voice: State<'_, Arc<VoiceService>>) -> Result<(), String> {
    voice.finish(Finish::Save).await;
    Ok(())
}

#[tauri::command]
pub async fn voice_cancel(
    app: AppHandle,
    voice: State<'_, Arc<VoiceService>>,
) -> Result<(), String> {
    voice.finish(Finish::Cancel).await;
    windows::hide(&app, windows::OVERLAY);
    Ok(())
}

/// Stop listening and return the transcript for editing (Tab in the overlay).
#[tauri::command]
pub async fn voice_edit(voice: State<'_, Arc<VoiceService>>) -> Result<Option<String>, String> {
    Ok(voice.finish(Finish::Edit).await)
}

#[tauri::command]
pub async fn voice_current(voice: State<'_, Arc<VoiceService>>) -> Result<VoiceState, String> {
    Ok(voice.current())
}

// ── Windows ─────────────────────────────────────────────────────────────

fn label(label: &str) -> Result<&'static str, String> {
    windows::floating_label(label).ok_or_else(|| format!("unknown panel {label}"))
}

#[tauri::command]
pub async fn panel_show(app: AppHandle, name: String) -> Result<(), String> {
    windows::show(&app, label(&name)?);
    Ok(())
}

#[tauri::command]
pub async fn panel_hide(app: AppHandle, name: String) -> Result<(), String> {
    windows::hide(&app, label(&name)?);
    Ok(())
}

#[tauri::command]
pub async fn panel_toggle(app: AppHandle, name: String) -> Result<(), String> {
    windows::toggle(&app, label(&name)?);
    Ok(())
}

#[tauri::command]
pub async fn main_show(app: AppHandle) -> Result<(), String> {
    windows::show_main(&app);
    windows::hide(&app, windows::PANEL);
    Ok(())
}

// ── Reminders ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn reminders_active(reminders: State<'_, Arc<Reminders>>) -> Result<Vec<Toast>, String> {
    Ok(reminders.active())
}

#[tauri::command]
pub async fn reminder_dismiss(
    reminders: State<'_, Arc<Reminders>>,
    id: String,
) -> Result<(), String> {
    reminders.dismiss(&id);
    Ok(())
}

/// Snoozes by `minutes`, or until tomorrow at 09:00 local when `tomorrow`.
#[tauri::command]
pub async fn reminder_snooze(
    app: AppHandle,
    state: State<'_, AppState>,
    reminders: State<'_, Arc<Reminders>>,
    item_id: String,
    minutes: Option<i64>,
    tomorrow: Option<bool>,
) -> Result<(), String> {
    let tz = local_tz();
    let until = if tomorrow.unwrap_or(false) {
        let day = now().with_timezone(&tz).date_naive() + Duration::days(1);
        let nine = day.and_hms_opt(9, 0, 0).ok_or("bad date")?;
        tz.from_local_datetime(&nine)
            .earliest()
            .map(|d| d.with_timezone(&Utc))
            .ok_or("bad local time")?
    } else {
        now() + Duration::minutes(minutes.unwrap_or(60).clamp(1, 60 * 24 * 7))
    };
    {
        let conn = state.db.lock().map_err(lock_message)?;
        let patch = ItemPatch {
            snoozed_until: Some(Some(until)),
            ..Default::default()
        };
        db::items::update_item(&conn, &item_id, patch, now(), tz).map_err(to_message)?;
    }
    reminders.dismiss(&item_id);
    emit_items_changed(&app, vec![item_id]);
    Ok(())
}

/// Marks the reminder's item done (recurring items advance to the next time).
#[tauri::command]
pub async fn reminder_done(
    app: AppHandle,
    state: State<'_, AppState>,
    reminders: State<'_, Arc<Reminders>>,
    item_id: String,
) -> Result<(), String> {
    {
        let conn = state.db.lock().map_err(lock_message)?;
        db::items::complete_item(&conn, &item_id, now(), local_tz()).map_err(to_message)?;
    }
    reminders.dismiss(&item_id);
    emit_items_changed(&app, vec![item_id]);
    Ok(())
}

// ── UI settings ─────────────────────────────────────────────────────────

const UI_PREFIXES: [&str; 5] = ["ui.", "onboarding.", "orb.", "reminders.", "voice."];

fn ui_key(key: &str) -> Result<(), String> {
    if UI_PREFIXES.iter().any(|p| key.starts_with(p)) {
        Ok(())
    } else {
        Err(format!("setting {key} is not editable from the UI"))
    }
}

#[tauri::command]
pub async fn settings_get(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<String>, String> {
    ui_key(&key)?;
    let conn = state.db.lock().map_err(lock_message)?;
    get_setting(&conn, &key).map_err(to_message)
}

#[tauri::command]
pub async fn settings_set(
    app: AppHandle,
    state: State<'_, AppState>,
    reminders: State<'_, Arc<Reminders>>,
    key: String,
    value: String,
) -> Result<(), String> {
    ui_key(&key)?;
    {
        let conn = state.db.lock().map_err(lock_message)?;
        set_setting(&conn, &key, &value).map_err(to_message)?;
    }
    match key.as_str() {
        "orb.enabled" if value == "0" => {
            windows::hide(&app, windows::ORB);
            windows::hide(&app, windows::PANEL);
        }
        "orb.enabled" => windows::show(&app, windows::ORB),
        k if k.starts_with("reminders.") => reminders.nudge(),
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_ui_settings_are_editable_from_panels() {
        assert!(ui_key("orb.enabled").is_ok());
        assert!(ui_key("voice.autoStop").is_ok());
        assert!(ui_key("ai.provider").is_err());
    }
}
