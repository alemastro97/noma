//! Interface language for the strings the backend shows by itself: the
//! menu-bar menu and native notifications. Follows the `ui.language` setting
//! ("system" | "en" | "it"), the same one the windows use (src/i18n/locale.ts).

use std::sync::OnceLock;

use tauri::{AppHandle, Emitter, Manager};

use crate::db::settings::get_setting;
use crate::state::AppState;

/// Setting key written by Settings → Appearance → Interface language.
pub const LANGUAGE_SETTING: &str = "ui.language";
/// Sent to every window after the setting changes, with the new value.
pub const LANGUAGE_EVENT: &str = "ui:language";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    It,
}

impl Lang {
    /// "it", "it-IT", "it_IT.UTF-8" → Italian; anything else → English.
    fn from_tag(tag: &str) -> Lang {
        if tag.trim().to_ascii_lowercase().starts_with("it") {
            Lang::It
        } else {
            Lang::En
        }
    }

    /// The preference resolved against the system language.
    pub fn from_pref(pref: Option<&str>, system: impl FnOnce() -> Lang) -> Lang {
        match pref {
            Some("en") => Lang::En,
            Some("it") => Lang::It,
            _ => system(),
        }
    }

    // ── Menu bar ────────────────────────────────────────────────────────

    pub fn tray_voice(self) -> &'static str {
        match self {
            Lang::En => "Capture by voice",
            Lang::It => "Cattura con la voce",
        }
    }
    pub fn tray_capture(self) -> &'static str {
        match self {
            Lang::En => "Quick capture",
            Lang::It => "Cattura rapida",
        }
    }
    pub fn tray_open(self) -> &'static str {
        match self {
            Lang::En => "Open NOMA",
            Lang::It => "Apri NOMA",
        }
    }
    pub fn tray_orb(self) -> &'static str {
        match self {
            Lang::En => "Floating orb",
            Lang::It => "Sfera fluttuante",
        }
    }
    pub fn tray_pause(self) -> &'static str {
        match self {
            Lang::En => "Pause reminders",
            Lang::It => "Metti in pausa i promemoria",
        }
    }
    pub fn tray_quit(self) -> &'static str {
        match self {
            Lang::En => "Quit NOMA",
            Lang::It => "Esci da NOMA",
        }
    }

    // ── Notifications ───────────────────────────────────────────────────

    pub fn reminder_body(self) -> &'static str {
        match self {
            Lang::En => "Reminder from NOMA",
            Lang::It => "Promemoria da NOMA",
        }
    }
    pub fn away_title(self) -> &'static str {
        match self {
            Lang::En => "While you were away",
            Lang::It => "Mentre eri via",
        }
    }
    pub fn away_body(self, n: usize) -> String {
        match self {
            Lang::En => format!("{n} reminders while you were away"),
            Lang::It => format!("{n} promemoria mentre eri via"),
        }
    }
    pub fn brief_title(self) -> &'static str {
        match self {
            Lang::En => "Today in NOMA",
            Lang::It => "Oggi in NOMA",
        }
    }
    pub fn brief_body(self, first: &str, more: usize) -> String {
        match self {
            Lang::En => format!("{first} and {more} more"),
            Lang::It if more == 1 => format!("{first} e un'altra cosa"),
            Lang::It => format!("{first} e altre {more} cose"),
        }
    }
    pub fn recap_title(self) -> &'static str {
        match self {
            Lang::En => "Your day in NOMA",
            Lang::It => "La tua giornata in NOMA",
        }
    }
    pub fn recap_body(self, done: usize, open: usize) -> String {
        match self {
            Lang::En => format!("{done} done, {open} still open"),
            Lang::It => {
                let d = if done == 1 { "fatta" } else { "fatte" };
                let o = if open == 1 {
                    "ancora aperta"
                } else {
                    "ancora aperte"
                };
                format!("{done} {d}, {open} {o}")
            }
        }
    }
    pub fn focus_done(self) -> &'static str {
        match self {
            Lang::En => "Focus session done",
            Lang::It => "Sessione di focus finita",
        }
    }

    /// "Due in 1 hour" for a "remind me before" notification, in the largest whole unit.
    pub fn due_in(self, minutes: i64) -> String {
        let (n, unit) = if minutes % (60 * 24 * 7) == 0 {
            (minutes / (60 * 24 * 7), 3)
        } else if minutes % (60 * 24) == 0 {
            (minutes / (60 * 24), 2)
        } else if minutes % 60 == 0 {
            (minutes / 60, 1)
        } else {
            (minutes, 0)
        };
        let one = n == 1;
        match self {
            Lang::En => {
                let unit = ["minute", "hour", "day", "week"][unit];
                let s = if one { "" } else { "s" };
                format!("Due in {n} {unit}{s}")
            }
            Lang::It => {
                let (sing, plur) = [
                    ("minuto", "minuti"),
                    ("ora", "ore"),
                    ("giorno", "giorni"),
                    ("settimana", "settimane"),
                ][unit];
                format!("Scade tra {n} {}", if one { sing } else { plur })
            }
        }
    }
}

/// The system's preferred language (macOS: first of AppleLanguages; else $LANG).
fn system_lang() -> Lang {
    static SYSTEM: OnceLock<Lang> = OnceLock::new();
    *SYSTEM.get_or_init(|| {
        #[cfg(target_os = "macos")]
        if let Ok(out) = std::process::Command::new("defaults")
            .args(["read", "-g", "AppleLanguages"])
            .output()
            && let Some(first) = String::from_utf8_lossy(&out.stdout)
                .split('"')
                .nth(1)
                .map(str::to_owned)
        {
            return Lang::from_tag(&first);
        }
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
            .map(|v| Lang::from_tag(&v))
            .unwrap_or(Lang::En)
    })
}

/// The interface language right now.
pub fn current(app: &AppHandle) -> Lang {
    let pref = app.try_state::<AppState>().and_then(|state| {
        let conn = state.db.lock().ok()?;
        get_setting(&conn, LANGUAGE_SETTING).ok().flatten()
    });
    Lang::from_pref(pref.as_deref(), system_lang)
}

/// After the setting changed: relabel the menu bar and tell every window.
pub fn on_changed(app: &AppHandle, value: &str) {
    crate::tray::relabel(app);
    let _ = app.emit(LANGUAGE_EVENT, value);
}

#[cfg(test)]
mod tests {
    use super::Lang;

    #[test]
    fn preference_resolves_against_the_system() {
        assert_eq!(Lang::from_pref(Some("it"), || Lang::En), Lang::It);
        assert_eq!(Lang::from_pref(Some("en"), || Lang::It), Lang::En);
        assert_eq!(Lang::from_pref(Some("system"), || Lang::It), Lang::It);
        assert_eq!(Lang::from_pref(None, || Lang::En), Lang::En);
        assert_eq!(Lang::from_tag("it_IT.UTF-8"), Lang::It);
        assert_eq!(Lang::from_tag("en-GB"), Lang::En);
    }

    #[test]
    fn due_in_uses_the_largest_whole_unit() {
        assert_eq!(Lang::It.due_in(60), "Scade tra 1 ora");
        assert_eq!(Lang::It.due_in(2 * 24 * 60), "Scade tra 2 giorni");
        assert_eq!(Lang::It.due_in(15), "Scade tra 15 minuti");
        assert_eq!(Lang::En.due_in(7 * 24 * 60), "Due in 1 week");
    }
}
