# Desktop presence

**Status:** Shipped (0.3.0) · **Phase:** 3

## Scope

NOMA lives on top of your work, not inside a window (design `screens/Desktop.html`).

1. **Voice (⌥ Space anywhere, rebindable).** **Hold** it and talk — releasing after ≥ 600 ms saves; or tap to start and tap again to finish. Audio from the default input via cpal, resampled to 16 kHz mono; an optional energy VAD (off by default; Settings → Voice → _Stop when I stop talking_) ends the utterance after 2.2 s of silence (min speech 350 ms, max 60 s). Transcription runs locally with **whisper.cpp** (whisper-rs, Metal): default **Whisper Large v3 Turbo q5_0** (574 MB), light option **Whisper Small q5_1**, pinned by URL + SHA-256 in `models.json`. Partial transcripts every 0.9 s over the last 15 s; the final transcript is cleaned of bracketed tags and known hallucinations and goes through the same pipeline as typed captures (`source = "voice"`). The model is loaded (and warmed) the moment ⌥ Space is pressed — while the user is still speaking, so nothing waits — and freed after 3 min idle (~0.6 GB back). The spoken language is pinned (`voice.language`: `it` default, `en`, `auto`) — auto-detect misfires on short phrases — and the encoder window is shrunk to the clip length (`audio_ctx`), cutting a 3 s utterance from 2.7 s to ~0.9 s on an M2. Smoke test without a microphone: `cargo run --release --example voice_check -- <model.bin> [it|en|auto]`.
2. **While talking (`voice.display`).** Default **Orb only**: the floating orb appears (even over NOMA or with the orb switched off) and pulses while listening; click it or release/tap ⌥ Space to finish. Optional **Full panel**: the overlay below. In both modes the window closes the moment you stop talking — transcription and AI capture run in the background, with only a short success flash on the orb; empty speech, missing model and errors still show the panel for a moment. Partials and the waveform exist only in the full panel (Whisper on the GPU while the UI animates caused stutter).
   **Voice overlay** (full panel). Glass panel at 18 % from the top of the active screen: listening orb, live waveform from input level, live transcript, and **↵** closes it at once. Keys: **↵** save now, **Tab** edit the transcript (then ↵ saves), **Esc** cancel. Messages for empty speech, missing model and errors.
3. **Quick capture (⌘⇧N anywhere).** Glass input that accepts focus without activating the main window; Auto mode; ↵ saves, Esc closes.
4. **Floating orb.** Non-activating NSPanel that floats over fullscreen apps on every Space (ADR-0002); shown when the main window is hidden or unfocused, hidden when it is in front. Click toggles the **panel** (Today with check-off, n left, _Talk to NOMA_, _Open NOMA_); drag moves it. Mirrors voice / AI state. Off via Settings → On your desktop.
5. **Reminders.** A scheduler sleeps until the next due reminder (effective time `snoozed_until` or `remind_at`, max 30 s, nudged on every item change) and shows glass toasts top-right: _Snooze 1h_, _Tomorrow_ (09:00), _Done_. Reminders missed by more than 10 min (NOMA was closed or the Mac asleep) are grouped into one "While you were away" toast. A macOS notification is also sent when NOMA is not in front (setting). Recurring reminders advance on Done. Pause from the tray or Settings.
6. **Lifecycle.** Closing the main window hides it (NOMA keeps running as a menu-bar agent, no Dock icon); reopening from the Dock / tray / panel restores it. Tray menu: Talk, Quick capture, Open NOMA, Floating orb ✓, Pause reminders ✓, Quit.
7. **Onboarding.** First run: welcome → download the two local models (understanding + voice) → try ⌥ Space. Skippable; stored as `onboarding.done`.

8. **Rebindable shortcuts.** Settings → On your desktop: for _Talk to NOMA_ and _Quick capture_, **Change** opens a game-style recorder — hold your combination, it shows live, then **Save** (or Esc / Cancel). Needs a modifier plus a key (so typing can't be hijacked); duplicates and combinations owned by another app are rejected with a message; **Reset** restores ⌥ Space / ⌘⇧N. The global shortcuts are released while recording so the current one doesn't fire. Stored as `shortcut.voice` / `shortcut.capture` (`Alt+Space`, `Super+Shift+KeyN`: modifiers + `KeyboardEvent.code`); every on-screen hint reads the live value (`shortcuts:changed`).

## Data

Migration `0004`: `items.reminded_at`. Settings keys: `ai.speechModel`, `voice.autoStop`, `voice.language`, `voice.display`, `shortcut.voice`, `shortcut.capture`, `orb.enabled`, `reminders.paused`, `reminders.native`, `onboarding.done` (UI keys are written through a whitelisted `settings_set`). Events: `voice:state`, `voice:level`, `voice:partial`, `reminders:changed`.

## Decisions

- One process, several hidden panel windows created at start-up (show/hide is instant; no window creation on the hot path).
- Key panels (overlay, capture) can take keyboard focus without activating NOMA, so the app you were in stays in front.
- Voice reuses the text capture pipeline: one code path, one eval.

## Memory

At rest NOMA uses ~0.2 GB (dev build): the Rust core ~40 MB plus three webviews (main, orb, quick capture; 30–45 MB each). The overlay, panel and toast windows are created on first use. The speech model (~0.6 GB) lives only around a capture; the AI server (~3.1 GB, Qwen3 4B + 306 MiB KV) idles out after 3 min.

## Notes

- Building whisper.cpp needs `cmake`; on machines whose Command Line Tools lack libc++ headers, a local, git-ignored `src-tauri/.cargo/config.toml` with `[env] CXXFLAGS = "-isystem <SDK>/usr/include/c++/v1"` works around it (or reinstall the Command Line Tools).

## Changelog

- 2026-11-09 — created and shipped in 0.3.0.
