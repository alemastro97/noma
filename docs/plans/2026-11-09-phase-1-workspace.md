# NOMA Phase 1 — Workspace & Views Implementation Plan

> Executed inline in one batch (owner preference: write large batches, verify once at the end). Spec: [`docs/specs/workspace-views.md`](../specs/workspace-views.md); design doc §5.1–5.2.

**Goal:** Users can create, see, arrange, connect, edit, complete and find cards by hand in Canvas, List and Kanban views.

**Architecture:** Rust gains tags, links, progress, duplicate, `includeDone` and auto-slot placement; the frontend gains a `Card` component family, three views under `src/views/`, a board store for placements and links, a detail panel, quick add, a ⌘K palette and a toast store. `@xyflow/react` renders the canvas with custom node and edge types.

## Global Constraints

Same as Phase 0 (tokens only, Lucide 1.75, English copy in `src/copy.ts`, reduced motion respected, no remote fonts), plus: canvas pan/zoom/drag stays at 60 fps with 500 cards (`onlyRenderVisibleElements`, memoized nodes).

## Review Focus

1. Deleting then pressing Undo within 5 s restores the card with no server call; after 5 s the delete is sent once.
2. Dragging a card on a filter canvas (Inbox) must not write a placement.
3. Typing "n" inside an input must not open quick add.
4. Moving a recurring item to Done in Kanban advances it instead of hiding it.
5. A link whose endpoint is deleted disappears (cascade) and the canvas does not crash on a dangling edge.

## Tasks (one batch)

1. **Backend** — migration 0002; `Item.endsAt/tags/linkCount`; `set_item_tags`; `links` module (create/list for board/delete, no self-links, unique pair); `project_progress`; `duplicate_item`; `list_items(.., include_done)`; auto-slot placement; project change moves placement; debug-only `dev_seed`; commands for all. Rust tests per module.
2. **Lib** — `src/lib/dates.ts` (`formatDue`, `formatReminder`, `relativeFromNow`, `eventBlock`, `timeRange`, `dayGroup`, `partOfDay`), `src/lib/recurrence.ts` (`RECURRENCE_PRESETS`, `describeRule`). Tests.
3. **Stores** — items (`load(filter, {includeDone})`, `duplicate`, `setStatus`, undoable `remove`), `board` (placements, links, move, connect, disconnect), `ui` (+ `selectedId`, `detailId`, `quickAdd`, `paletteOpen`), `toasts`. Tests for undo delete.
4. **Card** — `Card` with six variants + states + hover actions, `Checkbox`, `PriorityFlag`, `TagChip`. Tests: renders each type, completed state, checkbox calls complete.
5. **Views** — `CanvasView` (+ `CardNode`, `FlowEdge`, zoom pill), `ListView` (+ `groupItems`), `KanbanView` (+ `groupByStatus`, native drag and drop). Tests on grouping functions.
6. **Overlays** — `DetailPanel`, `QuickAdd`, `CommandPalette` (+ `filterCommands`), `Toasts`. Tests on palette filtering.
7. **Shell wiring** — Workspace picks the view; Header capture/search open overlays; Sidebar "+" for projects and boards; global hotkeys.
8. **Verify once** — typecheck, lint, format, vitest, cargo fmt/clippy/test, then run the app with `dev_seed` and compare against the mockups; fix everything in one pass; commit; push.
