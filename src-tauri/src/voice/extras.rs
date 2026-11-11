//! Small voice conveniences around a capture: the personal vocabulary Whisper
//! is primed with, the confirmation sound, spoken answers (`say`) and keeping
//! the original audio as a WAV file. See docs/specs/voice.md.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use rusqlite::Connection;
use tauri::{AppHandle, Manager};

use super::audio::SAMPLE_RATE;
use crate::db::{self, settings::get_setting};
use crate::state::{AppState, now};

pub const VOCABULARY_KEY: &str = "voice.vocabulary";
pub const SOUND_KEY: &str = "voice.sound";
pub const SPEAK_KEY: &str = "voice.speak";
pub const KEEP_AUDIO_KEY: &str = "voice.keepAudio";
/// Whisper's prompt is capped at ~224 tokens; stay well under it.
const PROMPT_MAX_CHARS: usize = 600;
const SAVED_SOUND: &str = "/System/Library/Sounds/Glass.aiff";

/// Reads a setting with a default ("1"/"0" switches and free text alike).
pub fn setting(app: &AppHandle, key: &str) -> Option<String> {
    let state = app.try_state::<AppState>()?;
    let conn = state.db.lock().ok()?;
    get_setting(&conn, key).ok().flatten()
}

fn switch(app: &AppHandle, key: &str, default_on: bool) -> bool {
    match setting(app, key).as_deref() {
        Some("1") => true,
        Some("0") => false,
        _ => default_on,
    }
}

// ── Personal vocabulary ─────────────────────────────────────────────────

/// Splits the user's word list (one per line or comma separated).
pub fn parse_words(list: &str) -> Vec<String> {
    list.split([',', '\n', ';'])
        .map(|w| w.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|w| !w.is_empty())
        .collect()
}

/// The initial prompt: names Whisper should expect, deduplicated, capped.
pub fn build_prompt(names: &[String]) -> Option<String> {
    let mut seen: Vec<String> = Vec::new();
    let mut out = String::new();
    for name in names {
        let key = name.to_lowercase();
        if name.trim().is_empty() || seen.contains(&key) {
            continue;
        }
        let sep = if out.is_empty() { "" } else { ", " };
        if out.len() + sep.len() + name.len() > PROMPT_MAX_CHARS {
            break;
        }
        out.push_str(sep);
        out.push_str(name.trim());
        seen.push(key);
    }
    (!out.is_empty()).then(|| format!("{out}."))
}

/// User words first (they are the reason this exists), then projects and boards.
pub fn vocabulary(conn: &Connection) -> Vec<String> {
    let mut names = get_setting(conn, VOCABULARY_KEY)
        .ok()
        .flatten()
        .map(|l| parse_words(&l))
        .unwrap_or_default();
    if let Ok(projects) = db::projects::list_projects(conn) {
        names.extend(projects.into_iter().map(|p| p.name));
    }
    if let Ok(boards) = db::boards::list_boards(conn) {
        names.extend(
            boards
                .into_iter()
                .filter(|b| b.kind == db::models::BoardKind::Personal)
                .map(|b| b.name),
        );
    }
    names
}

pub fn prompt_for(app: &AppHandle) -> Option<String> {
    let state = app.try_state::<AppState>()?;
    let conn = state.db.lock().ok()?;
    build_prompt(&vocabulary(&conn))
}

// ── Sounds and speech ───────────────────────────────────────────────────

/// Runs a command without waiting for it; a thread reaps it so no zombie is left.
fn spawn_detached(mut cmd: Command) {
    cmd.stdout(Stdio::null()).stderr(Stdio::null());
    match cmd.spawn() {
        Ok(mut child) => {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Err(e) => log::warn!("could not run {:?}: {e}", cmd.get_program()),
    }
}

/// The short "saved" sound (on by default, `voice.sound`).
pub fn play_saved(app: &AppHandle) {
    if cfg!(target_os = "macos") && switch(app, SOUND_KEY, true) {
        let mut cmd = Command::new("afplay");
        cmd.arg(SAVED_SOUND);
        spawn_detached(cmd);
    }
}

/// Whether answers are spoken aloud (on by default, `voice.speak`).
pub fn speak_enabled(app: &AppHandle) -> bool {
    cfg!(target_os = "macos") && switch(app, SPEAK_KEY, true)
}

/// Picks an installed voice for `lang` from `say -v ?` output
/// ("Alice               it_IT    # Ciao! Mi chiamo Alice.").
pub fn voice_for(list: &str, lang: &str) -> Option<String> {
    let prefix = format!("{lang}_");
    list.lines().find_map(|line| {
        let (head, _) = line.split_once('#')?;
        let mut parts: Vec<&str> = head.split_whitespace().collect();
        let locale = parts.pop()?;
        (locale.starts_with(&prefix) || locale.starts_with(&format!("{lang}-")))
            .then(|| parts.join(" "))
            .filter(|n| !n.is_empty())
    })
}

/// Says `text` with a voice matching `lang` ("it", "en"); the system voice if
/// none is installed. Never blocks the caller.
pub fn speak(text: &str, lang: &str) {
    if !cfg!(target_os = "macos") || text.trim().is_empty() {
        return;
    }
    let text = text.to_string();
    let lang = lang.to_string();
    std::thread::spawn(move || {
        let voice = Command::new("say")
            .args(["-v", "?"])
            .output()
            .ok()
            .and_then(|o| voice_for(&String::from_utf8_lossy(&o.stdout), &lang));
        let mut cmd = Command::new("say");
        if let Some(v) = voice {
            cmd.args(["-v", &v]);
        }
        cmd.arg(text);
        spawn_detached(cmd);
    });
}

// ── Original audio ──────────────────────────────────────────────────────

pub fn keep_audio(app: &AppHandle) -> bool {
    switch(app, KEEP_AUDIO_KEY, false)
}

pub fn recordings_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("recordings"))
}

/// 16-bit PCM mono WAV of 16 kHz float samples.
pub fn wav_bytes(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn write_wav(dir: &Path, file: &str, samples: &[f32]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join(file), wav_bytes(samples))
}

/// Saves the capture's audio and links it to the capture (best effort).
pub fn save_recording(app: &AppHandle, capture_id: &str, samples: &[f32]) {
    let Some(dir) = recordings_dir(app) else {
        return;
    };
    let id = db::voice::new_recording_id();
    let file = format!("{id}.wav");
    if let Err(e) = write_wav(&dir, &file, samples) {
        log::warn!("could not keep the voice recording: {e}");
        return;
    }
    let duration_ms = (samples.len() as i64 * 1000) / i64::from(SAMPLE_RATE);
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let Ok(conn) = state.db.lock() else { return };
    if let Err(e) = db::voice::insert_recording(&conn, &id, capture_id, &file, duration_ms, now()) {
        log::warn!("could not link the voice recording: {e}");
        let _ = std::fs::remove_file(dir.join(&file));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_dedupes_and_caps() {
        let names: Vec<String> = ["Marta", "Spring launch", "marta", " ", "Kyoto"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            build_prompt(&names).as_deref(),
            Some("Marta, Spring launch, Kyoto.")
        );
        assert_eq!(build_prompt(&[]), None);
        let many: Vec<String> = (0..200).map(|i| format!("Name{i}")).collect();
        assert!(build_prompt(&many).unwrap().len() <= PROMPT_MAX_CHARS + 1);
    }

    #[test]
    fn words_split_on_commas_and_lines() {
        assert_eq!(
            parse_words("Marta, Luca\n  Kyoto  station ;"),
            vec!["Marta", "Luca", "Kyoto station"]
        );
    }

    #[test]
    fn picks_a_voice_for_the_language() {
        let list = "Albert              en_US    # Hello! My name is Albert.\n\
                    Alice               it_IT    # Ciao! Mi chiamo Alice.\n\
                    Eddy (Italian (Italy)) it_IT # Ciao!\n";
        assert_eq!(voice_for(list, "it").as_deref(), Some("Alice"));
        assert_eq!(voice_for(list, "en").as_deref(), Some("Albert"));
        assert_eq!(voice_for(list, "fr"), None);
    }

    #[test]
    fn wav_header_is_valid() {
        let wav = wav_bytes(&[0.0, 0.5, -1.0]);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(wav.len(), 44 + 6);
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 16_000);
    }
}
