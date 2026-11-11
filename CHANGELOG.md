# Changelog

All notable changes to NOMA are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/) and versions follow [SemVer](https://semver.org/).

## Unreleased

## 0.4.0 — Second mind (2026-11-11)

- **Italian interface.** Settings → Appearance → _Interface language_: Match system, English or Italiano. Every window, the menu-bar menu and notifications switch live.
- **Accessibility pass.** Visible focus everywhere, labelled icon buttons, modal dialogs that keep focus inside and return it, keyboard-operable segmented settings and canvas group labels, Reduce motion respected by the graph and canvas zoom, and darker light-theme text tokens for 4.5:1 contrast.
- **Re-sort Inbox.** One button files captured Inbox cards again with your current projects, boards and corrections, and tells you how many moved.
- **Duplicate hints.** A capture that repeats an open card shows "Looks like …", with Merge or Keep both in the card details.
- **Break down.** Turn a task into 3–7 linked next-step tasks in the same place.
- **Project summary.** Summarize a project into a short status paragraph at the top of its view, regenerable.
- **Next action.** NOMA suggests the single next thing to do in a project, shown at the top of the project.
- **Weekly review.** A new Review page: done this week, overdue, forgotten (30+ days untouched) and a three-sentence summary.
- **Tags are reused.** Captures can pick up your existing tags; NOMA never invents new ones.
- **Bigger model option.** Qwen3 8B for Macs with 16 GB or more, with its memory need shown in Settings.
- **Search by meaning.** An optional small model (EmbeddingGemma, 334 MB, on your Mac) indexes your cards in the background; the ⌘K palette adds _Related_ cards that match what you mean, not only the exact words. Settings → Search by meaning.
- **Ask NOMA.** Ask a question about your cards (header _Ask_ button or the palette) and get a short answer from the local model, citing the cards it used; click a citation to open the card.
- **Checklists in cards.** Add, check, rename and reorder subtasks in the detail panel; cards show progress (`2/5`).
- **Attachments.** Attach files and images to a card (copied into NOMA's data folder), with image thumbnails, Open in Preview and Show in Finder.
- **Markdown notes.** Card descriptions render bold, italic, lists, task items, quotes, code and links; click to edit as plain text.
- **Archive and Trash.** Archive cards to hide them, delete to the Trash (restore or delete forever, Empty Trash); the Trash empties itself after 30 days.
- **Pinned cards.** Pin a card to keep it first in every view, with a small pin mark.
- **Manage projects and boards.** Right-click in the sidebar to rename, recolor, archive or delete (cards go to the Trash).
- **Multi-select and bulk actions.** ⌘/⇧-click (or ⌘A) to select many cards in any view; a bar lets you complete, move, date or delete them all at once.
- **Undo and redo everywhere.** ⌘Z / ⇧⌘Z undo card edits, completes, deletes, moves and canvas changes in the main window.
- **Filters and saved searches.** ⌘F filters any view by text, type, status, project, tag and date range; save a filter as a sidebar entry.
- **Drag to the sidebar.** Drop cards on the Inbox, a project or a personal board to move them there.
- **Canvas groups.** Draw a labelled frame on a board; cards inside move with it.
- **Links graph.** A new Graph view (⌘4) maps cards and their links.
- **Calendar view.** Month and week grids with cards on their date: drag a card to another day to reschedule it, click an empty day to add one.
- **Timeline view.** Two weeks side by side, with events stretching from start to end.
- **Custom snooze.** Reminder toasts offer _This evening_, _Next Monday_ and a small day/time picker.
- **Evening recap.** After 19:00, once a day, a toast with what you finished and what is still open (Settings → Time).
- **Focus timer.** Focus 25 minutes on a card from its details or the orb panel; the orb shows a progress ring and sessions are logged per card.
- **Habits.** Mark a repeating card as a habit to see its streak and last 7 days, also in a Habits section in Today.
- **Reminders before the due time.** 10 min, 1 hour, 1 day or 1 week before, set in the card's details.
- **Voice commands.** Say "segna fatto chiamare Marta", "sposta il dentista a domani" or "cosa ho oggi?" and NOMA acts on the card instead of taking a note, with Undo in the toast. Anything unclear is still captured.
- **Spoken answers.** "What's on today?" is read aloud in your voice language (Settings → Voice commands & sounds).
- **Confirmation sound** when a voice capture is saved.
- **Words to recognize.** Whisper now expects your project and board names plus your own list of names.
- **Keep the recording** (off by default): the audio of a voice capture is saved next to the card and can be played from the card's details.
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
- **Restore from backup.** Pick a snapshot in Settings → Your data; NOMA backs up the current state first, so a restore can be undone.
- **Import.** Markdown or text files, a Todoist CSV export, an Apple Notes folder or an Apple Reminders list, straight into the Inbox or a project.
- **Mirror to a folder.** Optionally keep a `noma.md` of all your cards in a folder you choose (an Obsidian vault, a synced folder), updated after each change.
- **`noma://` links.** `noma://capture?text=…`, `noma://open?item=…` and `noma://today` work from Shortcuts, Raycast, Alfred or a browser.
- **`noma` terminal command.** Install it from Settings: `noma buy milk tomorrow`, `noma today`, or pipe text into it.
- **MCP server.** `noma-mcp` lets AI assistants on your Mac search your cards, read Today and add captures; setup snippet in Settings.
- **Capture history.** Recent captures with their status, AI time and resulting card, and Retry for failed ones.
- **Your rhythm.** Captures and completed cards per week over the last 8 weeks, and the average AI time.

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
