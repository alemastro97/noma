# Workspace & views

**Status:** Shipped (0.1.0) · **Phase:** 1 · **Plan:** [../plans/2026-11-09-phase-1-workspace.md](../plans/2026-11-09-phase-1-workspace.md)

## Scope

Everything a user needs to work with cards by hand, before AI capture (Phase 2):

1. **Card system** — one shell, six types (task, reminder, note, idea, project, event), four states (default, hover, selected, completed), contextual actions. Pixel source: `design/screens/Cards.html`.
2. **Canvas view** — free-positioned cards on the 24 px dot grid, pan/zoom, drag with persisted positions, dashed animated connections created by dragging between cards, double-click empty space to add a card there. Source: `design/screens/Workspace.html`.
3. **List view** — rows grouped by time; Today groups by Morning / Afternoon / Evening / Checked. Source: `design/screens/Light.html`.
4. **Kanban view** — columns Inbox · Next · Doing · Done; drag a card between columns to change its status (Done completes it, recurring items advance).
5. **Detail panel** — slide-over to edit every field: title, type, body, status, priority, due, reminder, event end, repeat, project, tags; duplicate, delete.
6. **Quick add** — ⌘N / N / Capture button: title + type, lands in the current project or board. (Natural-language parsing arrives with AI in Phase 2.)
7. **⌘K palette** — commands (navigate, switch view, new card, new project/board, theme, sidebar) plus full-text search over cards.
8. **Projects and boards** — create from the sidebar; a project card shows its progress.
9. **Undoable delete** — deleting shows a toast with Undo for 5 s before the row is removed.

Non-goals for Phase 1: Calendar and Timeline views, AI parsing, reminders firing, multi-select drag on canvas.

## UX

- **Keyboard.** ⌘K palette · ⌘N quick add · N quick add (when not typing) · ⌘\ sidebar · on a selected card: E or ↵ open, ⌫ delete, ⌘D duplicate, Space toggle complete, Esc deselect / close panel. ⌘1/2/3 switch Canvas/Kanban/List.
- **Canvas in filter targets** (Inbox, Today, Upcoming, Ideas, Notes): cards are laid out in an automatic grid (not persisted). Positions and connections persist only on boards (a project's board or a personal board).
- **Motion.** New card fades + scales from .97 over 280 ms; hover lifts 2 px over 200 ms; panel slides 240 ms; connections dash-flow 1.6 s linear. All disabled under `prefers-reduced-motion`.
- **Empty states.** Copy from the design handoff per target.

## Data

Migration `0002`: `items.ends_at` (event end), `items.recurrence_start` (fixed series start so COUNT/UNTIL are measured from the first occurrence), and an FTS index keyed by item id (VACUUM-safe). Items now carry `tags` (names) and `linkCount` in the wire format. New commands: `items_duplicate`, `items_list` takes `includeDone`, `links_list(boardId)`, `links_create(from, to)`, `links_delete(id)`, `projects_progress()`, `dev_seed()` (debug builds only — loads the mockup's sample content into an empty database).

New items on a board get the first free cell of a 4-column grid (300 × 200 px) that no existing card overlaps. Recurrences are expanded in the user's local time zone. Today includes overdue open items; Upcoming starts tomorrow. All commands run off the main thread. Moving an item to another project moves it to that project's board.

## Open questions

None.

## Changelog

- 2026-11-09 — created
- 2026-11-09 — shipped in 0.1.0 together with the Phase 0 review fixes (recurrence COUNT/time zone, theme "system", stale loads, transactions, project moves, overdue in Today, non-fatal shortcut registration, FTS rowid).
- Dev tooling: `pnpm dev` in a plain browser runs against an in-memory mock backend (`src/dev/mockBackend.ts`) with the mockup's sample data.
