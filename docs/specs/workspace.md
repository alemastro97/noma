# Workspace power tools

**Status:** Shipped (Unreleased) · **Phase:** 4 · **Builds on:** [Workspace & views](workspace-views.md)

## Scope

1. **Multi-select.** ⌘-click toggles a card, ⇧-click selects the range from the last clicked card in on-screen order (list rows, Kanban columns, canvas nodes, graph dots), ⌘A selects every visible card, and on the canvas a ⇧-drag box selects too. With two or more cards selected a **bulk bar** floats above the toasts: Done · Move (Inbox, a project or a personal board) · Date (sets the due date to 09:00 that day, or clears it) · Delete · Clear. ⌫ deletes and Space completes the whole selection. Selected cards drag together on the canvas and between Kanban columns.
2. **Global undo / redo.** ⌘Z / ⇧⌘Z in the main window undo and redo card edits, completes, deletes, moves (bulk bar or drag to the sidebar), canvas position drags and group changes. Text fields keep their own native undo. A short toast says what was undone. The stack lives in memory (100 steps) and is lost when the window reloads.
3. **Filters and advanced search.** ⌘F or the filter button in the header opens a filter bar over the current view: text (title and notes), type, status, project (or "No project"), tag and a date range on the card's date (due, else reminder). Conditions combine with AND; within a type or status they combine with OR. **Save as filter** stores the conditions as a sidebar entry under _Filters_; opening it loads every card and applies them. Hover a saved filter to delete it.
4. **Drag to the sidebar.** Drag a card (or the selection it belongs to) from the list, Kanban or canvas onto the Inbox, a project or a personal board in the sidebar to move it there. The target highlights while hovered.
5. **Canvas groups.** On a board's canvas, the **Group** tool draws a labelled frame (drag a rectangle; the label opens for editing). Dragging the frame by its label moves every card whose center is inside it. Frames resize from their corners, rename on double-click and delete from the × on the label.
6. **Links graph.** A fourth view mode, **Graph** (⌘4): cards as dots colored by type and sized by link count, links as dashed lines, laid out by a small built-in force simulation (no dependency). Drag to pan, scroll to zoom, drag a dot to move it, click to select (⌘ / ⇧ work), double-click to open. _Only linked cards_ hides cards without links.

Non-goals for v1: nested groups, group membership stored in the database, undo across app restarts, saving edits back into an existing saved filter (save a new one instead).

## Data

Migration `0009_workspace.sql`:

- `saved_filters(id, name, spec, created_at)` — `spec` is the frontend's `FilterSpec` as JSON (`text`, `types`, `tags`, `projectIds` with `"none"` for no project, `statuses`, `from`, `to` as local `YYYY-MM-DD`).
- `canvas_groups(id, board_id → boards ON DELETE CASCADE, label, x, y, w, h, created_at)`; minimum size 80 × 80.

Commands (`src-tauri/src/commands/workspace.rs` over `db::workspace`):

| Command                                                                                                                 | Purpose                                                                                                                                                                                                                                   |
| ----------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `filters_list` / `filters_create(name, spec)` / `filters_delete(id)`                                                    | Saved filters                                                                                                                                                                                                                             |
| `groups_list(boardId)` / `groups_create(boardId, label, x, y, w, h)` / `groups_update(id, patch)` / `groups_delete(id)` | Canvas frames (no change event: the canvas already shows the change)                                                                                                                                                                      |
| `links_all()`                                                                                                           | Every link, for the graph                                                                                                                                                                                                                 |
| `items_move(ids, place)` → prior places                                                                                 | Bulk move to a project / personal board / Inbox in one transaction; returns where each card was, including its canvas spot                                                                                                                |
| `items_unmove(prior)`                                                                                                   | Undo of `items_move`                                                                                                                                                                                                                      |
| `items_snapshot(ids)` / `items_restore_rows(snapshots)`                                                                 | Undo of a delete after its 5 s window: the card's rows (item, tags, placement, links) are captured when the delete starts and re-inserted with the same id; links to cards that are gone, tags or boards that no longer exist are skipped |

Frontend: `stores/history.ts` (stack), `stores/selection.ts`, `stores/bulk.ts`, `stores/filters.ts`, `stores/groups.ts`, `lib/filterSpec.ts`, `lib/force.ts`, `views/graph/GraphView.tsx`, `views/canvas/FrameNode.tsx`, `components/BulkBar`, `components/FilterBar`, `components/Sidebar/SavedFilters.tsx` and `dropTarget.ts`. Browser dev mode: `src/dev/mock/workspace.ts`.

## Decisions

- **Undo replays through the normal paths.** Each history entry carries an inverse that calls the existing store actions or commands, so the backend stays the source of truth and change events refresh every window. The items store records edits, completes and deletes itself; a guard suppresses recording while an entry replays.
- **Deletes stay deferred.** The existing 5 s delete window is kept (toast Undo still works). ⌘Z first cancels a pending delete; after it has committed, the snapshot taken at delete time restores the card with its id, so later history entries that refer to it keep working.
- **Filters refine, they don't query.** Conditions apply client-side to what the view loaded (a saved filter loads every card). Simple, instant, and the same code serves every view. Navigating elsewhere clears the conditions so nothing stays silently hidden.
- **Group membership is geometric.** A card belongs to a frame while its center is inside it, computed when the drag starts — no extra table to keep in sync with card moves.
- **Graph without a library.** An O(n²) force layout is plenty for a personal graph of a few hundred cards and keeps the bundle unchanged.
- **Canvas drop on the sidebar** uses the pointer position at the end of the xyflow drag (`elementFromPoint`), since canvas drags are pointer drags, not HTML drag and drop.

## Open questions

None.

## Changelog

- 2026-11-11 — created and shipped: multi-select with bulk bar, global undo/redo, filter bar and saved filters, drag to sidebar, canvas groups, links graph.
- 2026-11-11 — integration: the row-restore command is `items_restore_rows` (`items_restore` belongs to Cards and untrashes a card). Undo of a single card delete after its window now restores the card from the Trash, since Delete moves cards to the Trash.
