# Voice extras

**Status:** Shipped (Unreleased) · **Area:** voice · builds on [desktop-presence.md](desktop-presence.md) §1–2

## Scope

1. **Voice commands.** When the transcript starts with a command cue (_segna, completa, ho fatto, fatto, finito, spunta, mark, complete, check off, done_ · _sposta, rimanda, posticipa, anticipa, move, postpone, reschedule, push_ · _cosa ho, che cosa ho, cosa c'è, what's on, what do I have_; a leading "NOMA," is ignored) NOMA asks the local model for the intent with a small `json_schema` (`intent` ∈ done / move / today / none, `target`, `date`). Supported:
   - **Mark done** — "segna fatto chiamare Marta", "mark call Marta done": completes the card (a repeating card advances, as with the Done button).
   - **Move** — "sposta il dentista a domani", "move the review to Friday at 10": the due date moves (its reminder and end move by the same amount); a card with only a reminder moves the reminder. A bare day keeps the card's time of day; no time at all → 09:00. Dates in the past or more than two years ahead are refused.
   - **What's on today?** — "cosa ho oggi?", "what's on today?" (short phrases with _oggi/today_ are recognized without the model): the Today list appears in the morning-brief toast and is optionally read aloud (§2).
     The target card is found with full-text search (`db::items::search_items`) among open cards; a second try drops words of ≤ 3 letters. Exactly one match, or one whose title equals the target, is acted on; **no match, several matches, an unknown intent, a model that is not ready or a model error all fall back to a normal capture**. Done and Move are confirmed in the toast window (_Marked done_ / _Moved to Thu 12 · 15:00_) with **Undo** (restores status and dates; hidden for a repeating card marked done) and **Open**; it closes itself after 7 s unless hovered.
2. **Spoken answers.** "What's on today?" is answered aloud with macOS `say`, using an installed voice whose locale matches the voice language (`voice.language`; with _auto_ the language Whisper detected; Italian unless English). Up to five titles are read, then "and N more". Settings → _Voice commands & sounds_ → _Answer aloud_ (`voice.speak`, on by default).
3. **Confirmation sound.** A saved voice capture plays `/System/Library/Sounds/Glass.aiff` via `afplay` (fire and forget). _Confirmation sound_ (`voice.sound`, on by default).
4. **Personal vocabulary.** Whisper gets an initial prompt: the user's words (`voice.vocabulary`, comma separated, edited in Settings), then project names and personal board names; deduplicated case-insensitively and capped at 600 characters (Whisper's prompt limit is ~224 tokens). Used for the final transcript and for live partials.
5. **Keep the original audio.** With _Keep the recording_ (`voice.keepAudio`, **off by default**) the 16 kHz mono audio of a voice capture is written as a 16-bit PCM WAV to `<app data>/recordings/<id>.wav` and linked to its capture. The card detail panel shows a _Recording_ row with a play button (duration as m:ss) for the card the capture produced. Commands never keep audio.

## Data

Migration `0011_voice.sql`: table `voice_recordings (id, capture_id → captures ON DELETE CASCADE, file, duration_ms, created_at)`. Settings keys: `voice.vocabulary`, `voice.sound`, `voice.speak`, `voice.keepAudio`. Commands: `voice_recording_for_item(itemId)`, `voice_recording_audio(id)` (raw WAV bytes, an `ArrayBuffer` in JS), `voice_command_undo(toastId)`. Command toasts reuse the reminder toast list with ids `voice:done:<item>` / `voice:moved:<item>` (no new `Toast` field); "today" reuses the `brief` toast. `AiService::complete_once` runs one schema-constrained completion outside the capture queue.

## Decisions

- **Cue first, model second.** The model is consulted only when the first words look like a command, so ordinary captures pay nothing; "today" phrases skip the model entirely.
- **Fail toward capturing.** A wrong action on a card is worse than an extra note, so anything uncertain becomes a capture (which the user can delete with the filed toast's Undo).
- **No new toast type.** Encoding the command in the toast id keeps `reminders::Toast` unchanged.
- **Undo is kept in memory** (the latest 20 commands): a toast lives for seconds, a restart forgets it.
- **Spoken text lives in Rust** (Italian and English), because it is generated per answer and depends on the voice language rather than the UI language.
- Audio is played back through the IPC as bytes and a blob URL: no asset-protocol scope needed.
- Recordings stay when their card is deleted (the capture keeps them; `ON DELETE CASCADE` follows the capture). A cleanup pass is a later improvement.

## Changelog

- 2026-11-11 — First version: voice commands (done, move, today), spoken answers, confirmation sound, personal vocabulary, kept recordings.
