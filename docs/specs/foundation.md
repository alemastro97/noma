# Foundation

**Status:** In progress · **Phase:** 0 · **Plan:** [../plans/2026-11-09-phase-0-foundation.md](../plans/2026-11-09-phase-0-foundation.md)

## Scope

1. Repository on `alemastro97/noma` (private), spec index, ADRs.
2. Tauri 2 + React 19 + Vite scaffold with Vitest and ESLint.
3. Design tokens (verbatim from the handoff), bundled Geist fonts, dark/light/system theme.
4. `NomaOrb` component with five states.
5. App shell: collapsible sidebar, header, view switcher, empty states, app icon.
6. Floating-window spike with global shortcut → go/no-go in ADR-0002.
7. SQLite data layer in Rust (items, projects, boards, placements, recurrence, FTS5).
8. Tauri commands, typed TypeScript API, Zustand stores.
9. CI (test) and release (macOS DMG; Windows/Linux experimental) workflows.
10. Release 0.0.1.

## UX

See design document §5.1 and `design/screens/Workspace.html`. Phase 0 ships the shell with empty states only; cards and views arrive in Phase 1.

## Data

Schema v1 (`src-tauri/src/db/migrations/0001_init.sql`), SQLite with WAL and foreign keys on. Timestamps are RFC 3339 UTC text; ids are UUID v7.

| Table               | Purpose                                                                                                                                                                                                                                                                    |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `items`             | Every card: `kind` (task, reminder, note, idea, project_ref, event), `title`, `body`, `status` (inbox, next, doing, done), `priority` (low, normal, high, urgent), `due_at`, `remind_at`, `snoozed_until`, `recurrence` (RRULE), `project_id`, timestamps, `completed_at`. |
| `projects`          | Name, color, owning `board_id`. Creating a project creates its board.                                                                                                                                                                                                      |
| `boards`            | Spatial canvases: `kind` project or personal.                                                                                                                                                                                                                              |
| `placements`        | One row per placed item (`item_id` PK, `board_id`, `x`, `y`). Items with no placement and no project are the Inbox.                                                                                                                                                        |
| `connections`       | Directed edges between items with optional label.                                                                                                                                                                                                                          |
| `tags`, `item_tags` | Free tags (case-insensitive unique).                                                                                                                                                                                                                                       |
| `captures`          | Raw capture text, model output, status (pending, processing, done, failed), latency, resulting item.                                                                                                                                                                       |
| `item_occurrences`  | Log of completed/skipped occurrences of recurring items.                                                                                                                                                                                                                   |
| `settings`          | Key/value.                                                                                                                                                                                                                                                                 |
| `items_fts`         | FTS5 external-content index over title and body, kept in sync by triggers.                                                                                                                                                                                                 |

Filters (`db::models::ItemFilter`): Inbox, Today (local day in the given tz, by `due_at` or `remind_at`), Upcoming (next N days), Ideas, Notes, Project, Board, All. All but All exclude `done`.

Recurrence subset: `FREQ=DAILY|WEEKLY|MONTHLY|YEARLY`, `INTERVAL`, `BYDAY`, `BYMONTHDAY`, `UNTIL`, `COUNT`, `WKST`. Completing a recurring item logs an occurrence and shifts `due_at`/`remind_at` to the next occurrence; an exhausted rule marks the item `done`.

## Open questions

None.

## Changelog

- 2026-11-09 — created
- 2026-11-09 — Tasks 1–7 landed: repo, scaffold, tokens, orb, shell, floating-window spike (ADR-0002, go), data layer
