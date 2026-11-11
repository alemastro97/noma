# Changelog

All notable changes to NOMA are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/) and versions follow [SemVer](https://semver.org/).

## Unreleased

- **Count in the menu bar:** the NOMA symbol shows how many things are left today (can be turned off).
- **The orb glows** in the reminder color when something is due within 15 minutes or overdue.
- **The orb remembers where you put it** on each screen and always stays on-screen.
- **Search and capture from the orb panel:** a full-text search field and a one-line capture field.
- **Quiet hours:** a daily window in which reminders wait (shown as "While you were away" afterwards) and the orb stays still.
- **Orb size (S/M/L) and style** (Lively, Calm, Faded) in Settings → On your desktop.
- **More global shortcuts:** Open Today, Search and Open last capture, off until you set them.

- **Filed-in toast.** After a voice or quick capture NOMA shows where the card went, with Undo, Move and Open.
- **NOMA learns from corrections.** Move a captured card and similar notes are filed the same way next time.
- **Morning brief.** Once a day, a small note with what is on today.
- **Backups and export.** A daily local backup (last 10 kept) and an export of everything as Markdown + JSON, in Settings → Your data.

- NOMA stays alive when you press ⌘Q (only the window closes; Quit NOMA in the menu bar exits), and an optional **Open at login** starts it quietly in the menu bar, so ⌥ Space always opens the orb. Installed builds turn it **on by default at first launch** (once; Settings stays in charge).

- Captures get a **short headline title**; the card description keeps what you said, plus an AI summary for longer notes.
- NOMA now files a capture in the **right project or personal board** by topic, using what each already contains; when unsure it goes to the Inbox.
- New cards are **linked automatically** to clearly related cards.
- Much lighter at rest (~1 GB → ~0.2 GB): the speech model loads while you speak and is released after 3 minutes, the AI server after 3 minutes, its cache is halved, and the rarely used windows are created on first use. Quality unchanged.
- Voice: hold-to-talk by default, rebindable shortcuts with a live recorder, orb-only mode.

## 0.3.0 — Desktop presence (2026-11-09)

- **Talk to NOMA anywhere:** hold or tap ⌥ Space, speak, and the card is created — transcribed on your Mac by Whisper (Large v3 Turbo, 574 MB) with live transcript, waveform and a summary of what was captured. ↵ save · Tab edit · Esc cancel.
- **Quick capture anywhere** with ⌘⇧N, without leaving the app you're in.
- **Floating orb** over every app and fullscreen Space; click it for today's list, a mic button and Open NOMA.
- **Reminders that reach you:** glass toasts with Snooze 1h / Tomorrow / Done, grouped "while you were away" catch-up, macOS notifications when NOMA is in the background, pause from the menu bar.
- NOMA keeps running in the menu bar when you close the window.
- First-run onboarding with the model downloads; Settings → Voice and On your desktop.

## 0.2.0 — On-device AI capture (2026-11-09)

- Quick add has an **Auto** mode: say or type anything and NOMA turns it into one to three typed cards — dates, reminders, repeats, priority and project included — with a model running entirely on your Mac.
- Settings → On-device AI: download Qwen3 4B (2.5 GB) or the lighter Qwen3 1.7B, with progress, cancel and remove; optional Ollama engine for development.
- The capture card appears instantly and is rewritten in place when the model is done; captures made before a model is installed wait and run later; failures keep your words as a note with Retry.
- Review chips on cards when the model was unsure: check date, new project?, check this.
- Extraction eval (40 Italian + English notes): type 97.5 %, date 100 %, warm p95 2.1 s on an M2.
- Fix: canvas cards could disappear after any change until the window was resized.

## 0.1.0 — Workspace & views (2026-11-09)

- Card system: task, reminder, note, idea, project and event cards with default, hover, selected and completed states and hover actions (open, duplicate, delete).
- Canvas view: free-positioned cards with persisted positions, pan/zoom, dashed animated connections made by dragging between cards, double-click to add a card in place; only visible cards render (60 fps with 500 cards).
- List view grouped by part of day (Today) or by date, with a Checked section; Kanban view with drag between Inbox, Next, Doing and Done.
- Detail panel to edit every field, including repeat presets, project and tags.
- Quick add (⌘N / N / Capture), ⌘K command palette with full-text search, keyboard shortcuts on the selected card, undoable delete.
- Create projects and personal boards from the sidebar; project cards show progress.
- Fixes from the Phase 0 review: recurring COUNT and local time zone, theme following the OS after switching to "system", out-of-order list loads, atomic multi-step writes, moving cards between projects, overdue items in Today, NOMA starting even when ⌥ Space is taken, VACUUM-safe search index; commands no longer block the UI thread.

## 0.0.1 — Foundation (2026-11-09)

- Design document, Phase 0 plan, design handoff, spec index and ADRs.
- Tauri 2 + React 19 + Vite scaffold with Vitest, ESLint and Prettier.
- Design tokens copied verbatim from the handoff, bundled Geist / Geist Mono, dark / light / system theme.
- `NomaOrb` component with idle, listening, processing, success and error states.
- App shell: collapsible sidebar (⌘\), header with view switcher, search, capture button and AI status, empty states, app icon.
- Floating orb as a non-activating NSPanel toggled with ⌥ Space — visible over fullscreen apps and on every Space (ADR-0002).
- SQLite data layer: items, projects, boards, placements, recurrence (RRULE subset), FTS5 search.
- Tauri commands, typed TypeScript API and Zustand stores; sidebar counts are live.
- CI (typecheck, lint, tests, clippy) and release workflow (universal macOS DMG; Windows/Linux experimental).
