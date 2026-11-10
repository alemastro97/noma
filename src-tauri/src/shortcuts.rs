//! User-configurable global shortcuts. Accelerators are stored as strings like
//! `Alt+Space` or `Super+Shift+KeyN`: modifiers (`Alt`, `Control`, `Shift`,
//! `Super`) then a `KeyboardEvent.code`, which the settings recorder produces.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

use crate::db::settings::{get_setting, set_setting};
use crate::state::AppState;

pub const SHORTCUTS_CHANGED: &str = "shortcuts:changed";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Voice,
    Capture,
}

impl Action {
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "voice" => Ok(Self::Voice),
            "capture" => Ok(Self::Capture),
            other => Err(format!("unknown shortcut action {other}")),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Voice => "voice",
            Self::Capture => "capture",
        }
    }

    pub fn default_accelerator(self) -> &'static str {
        match self {
            Self::Voice => "Alt+Space",
            Self::Capture => "Super+Shift+KeyN",
        }
    }

    fn key(self) -> String {
        format!("shortcut.{}", self.name())
    }
}

/// Parses `Alt+Shift+KeyN`. Requires at least one modifier so a bare key
/// can never steal typing system-wide.
pub fn parse(accelerator: &str) -> Result<Shortcut, String> {
    let mut mods = Modifiers::empty();
    let mut code = None;
    for part in accelerator.split('+').map(str::trim) {
        match part {
            "Alt" => mods |= Modifiers::ALT,
            "Control" => mods |= Modifiers::CONTROL,
            "Shift" => mods |= Modifiers::SHIFT,
            "Super" => mods |= Modifiers::SUPER,
            key => {
                if code.is_some() {
                    return Err("use one key plus modifiers".into());
                }
                code = Some(Code::from_str(key).map_err(|_| format!("unknown key {key}"))?);
            }
        }
    }
    let code = code.ok_or("press a key together with the modifiers")?;
    if mods.is_empty() {
        return Err("include ⌥, ⌘, ⌃ or ⇧ so it doesn't hijack typing".into());
    }
    Ok(Shortcut::new(Some(mods), code))
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ShortcutMap {
    pub voice: String,
    pub capture: String,
}

pub struct Shortcuts {
    current: Mutex<HashMap<Action, (String, Shortcut)>>,
}

impl Shortcuts {
    /// Reads the saved accelerators, falling back to defaults when missing or invalid.
    pub fn load(app: &AppHandle) -> Self {
        let mut current = HashMap::new();
        for action in [Action::Voice, Action::Capture] {
            let saved = app.try_state::<AppState>().and_then(|s| {
                s.db.lock()
                    .ok()
                    .and_then(|c| get_setting(&c, &action.key()).ok().flatten())
            });
            let entry = saved
                .and_then(|acc| parse(&acc).ok().map(|s| (acc, s)))
                .unwrap_or_else(|| {
                    let acc = action.default_accelerator();
                    (acc.to_string(), parse(acc).expect("default shortcut"))
                });
            current.insert(action, entry);
        }
        Self {
            current: Mutex::new(current),
        }
    }

    pub fn map(&self) -> ShortcutMap {
        let cur = self.current.lock().expect("shortcuts");
        ShortcutMap {
            voice: cur[&Action::Voice].0.clone(),
            capture: cur[&Action::Capture].0.clone(),
        }
    }

    /// Which action a pressed shortcut belongs to.
    pub fn action_for(&self, shortcut: &Shortcut) -> Option<Action> {
        self.current
            .lock()
            .expect("shortcuts")
            .iter()
            .find(|(_, (_, s))| s == shortcut)
            .map(|(a, _)| *a)
    }

    /// Registers every shortcut; another app may own one, which only logs.
    pub fn register_all(&self, app: &AppHandle) {
        for (action, (acc, shortcut)) in self.current.lock().expect("shortcuts").iter() {
            if let Err(e) = app.global_shortcut().register(*shortcut) {
                log::warn!("could not register {} ({acc}): {e}", action.name());
            }
        }
    }

    /// Frees every shortcut while the settings recorder listens for keys.
    pub fn suspend(&self, app: &AppHandle) {
        let _ = app.global_shortcut().unregister_all();
    }

    /// Changes one shortcut. Fails without side effects if it is invalid, equal
    /// to the other action's, or already taken by another app.
    pub fn set(&self, app: &AppHandle, action: Action, accelerator: &str) -> Result<(), String> {
        let shortcut = parse(accelerator)?;
        {
            let cur = self.current.lock().expect("shortcuts");
            if let Some((other, _)) = cur
                .iter()
                .find(|(a, (_, s))| **a != action && *s == shortcut)
            {
                return Err(format!("already used for {}", other.name()));
            }
        }
        // Probe the combination so one owned by another app is reported
        // instead of silently dead; nothing changes if it fails.
        let gs = app.global_shortcut();
        let _ = gs.unregister_all();
        let probe = gs.register(shortcut);
        let _ = gs.unregister(shortcut);
        if let Err(e) = probe {
            self.register_all(app);
            return Err(format!("couldn't use it — another app may own it ({e})"));
        }
        self.current
            .lock()
            .expect("shortcuts")
            .insert(action, (accelerator.to_string(), shortcut));
        self.register_all(app);
        self.persist(app, action, accelerator);
        let _ = app.emit(SHORTCUTS_CHANGED, self.map());
        Ok(())
    }

    pub fn reset(&self, app: &AppHandle, action: Action) -> Result<(), String> {
        self.set(app, action, action.default_accelerator())
    }

    fn persist(&self, app: &AppHandle, action: Action, accelerator: &str) {
        if let Some(state) = app.try_state::<AppState>()
            && let Ok(conn) = state.db.lock()
        {
            let _ = set_setting(&conn, &action.key(), accelerator);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modifiers_and_code() {
        let s = parse("Super+Shift+KeyN").unwrap();
        assert_eq!(
            s,
            Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyN)
        );
        assert_eq!(
            parse("Alt+Space").unwrap(),
            Shortcut::new(Some(Modifiers::ALT), Code::Space)
        );
    }

    #[test]
    fn rejects_bare_keys_and_nonsense() {
        assert!(parse("KeyN").is_err());
        assert!(parse("Alt").is_err());
        assert!(parse("Alt+Nope").is_err());
        assert!(parse("Alt+KeyA+KeyB").is_err());
    }

    #[test]
    fn defaults_parse() {
        for a in [Action::Voice, Action::Capture] {
            assert!(parse(a.default_accelerator()).is_ok());
        }
    }
}
