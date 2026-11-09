# Desktop presence

**Status:** Shipped (0.3.0) · **Phase:** 3

## Scope

NOMA lives on top of your work, not inside a window (design `screens/Desktop.html`).

1. **Voice (⌥ Space anywhere).** Press to toggle, or hold ≥ 600 ms for push-to-talk (release saves). Audio from the default input via cpal, resampled to 16 kHz mono; an energy VAD ends the utterance after 1.4 s of silence (min speech 350 ms, max 60 s; disable with Settings → Voice → _Stop when I stop talking_). Transcription runs locally with **whisper.cpp** (whisper-rs, Metal): default **Whisper Large v3 Turbo q5_0** (574 MB), light option **Whisper Small q5_1**, pinned by URL + SHA-256 in `models.json`. Partial transcripts every 0.9 s over the last 15 s; the final transcript is cleaned of bracketed tags and known hallucinations and goes through the same pipeline as typed captures (`source = "voice"`). Model loads on demand and unloads after 60 s idle.
2. **Voice overlay.** Glass panel at 18 % from the top of the active screen: listening orb, live waveform from input level, live transcript, then a summary of the captured card (type, project, date, reminder, repeat) that closes itself after ~1.8 s. Keys: **↵** save now, **Tab** edit the transcript (then ↵ saves), **Esc** cancel. Messages for empty speech, missing model and errors.
3. **Quick capture (⌘⇧N anywhere).** Glass input that accepts focus without activating the main window; Auto mode; ↵ saves, Esc closes.
4. **Floating orb.** Non-activating NSPanel that floats over fullscreen apps on every Space (ADR-0002); shown when the main window is hidden or unfocused, hidden when it is in front. Click toggles the **panel** (Today with check-off, n left, _Talk to NOMA_, _Open NOMA_); drag moves it. Mirrors voice / AI state. Off via Settings → On your desktop.
5. **Reminders.** A scheduler sleeps until the next due reminder (effective time `snoozed_until` or `remind_at`, max 30 s, nudged on every item change) and shows glass toasts top-right: _Snooze 1h_, _Tomorrow_ (09:00), _Done_. Reminders missed by more than 10 min (NOMA was closed or the Mac asleep) are grouped into one "While you were away" toast. A macOS notification is also sent when NOMA is not in front (setting). Recurring reminders advance on Done. Pause from the tray or Settings.
6. **Lifecycle.** Closing the main window hides it (NOMA keeps running as a menu-bar agent, no Dock icon); reopening from the Dock / tray / panel restores it. Tray menu: Talk, Quick capture, Open NOMA, Floating orb ✓, Pause reminders ✓, Quit.
7. **Onboarding.** First run: welcome → download the two local models (understanding + voice) → try ⌥ Space. Skippable; stored as `onboarding.done`.

## Data

Migration `0004`: `items.reminded_at`. Settings keys: `ai.speechModel`, `voice.autoStop`, `orb.enabled`, `reminders.paused`, `reminders.native`, `onboarding.done` (UI keys are written through a whitelisted `settings_set`). Events: `voice:state`, `voice:level`, `voice:partial`, `reminders:changed`.

## Decisions

- One process, several hidden panel windows created at start-up (show/hide is instant; no window creation on the hot path).
- Key panels (overlay, capture) can take keyboard focus without activating NOMA, so the app you were in stays in front.
- Voice reuses the text capture pipeline: one code path, one eval.

## Notes

- Building whisper.cpp needs `cmake`; on machines whose Command Line Tools lack libc++ headers, a local, git-ignored `src-tauri/.cargo/config.toml` with `[env] CXXFLAGS = "-isystem <SDK>/usr/include/c++/v1"` works around it (or reinstall the Command Line Tools).

## Changelog

- 2026-11-09 — created and shipped in 0.3.0.
