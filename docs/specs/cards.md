# Cards

**Status:** Shipped (Unreleased) · **Phase:** 4

## Scope

What a card can hold beyond its title, and where cards go when you are done with them.

1. **Checklist.** A card has an ordered list of subtasks (own table). In the detail panel: add (↵), check, rename inline, move up/down, remove. Cards show a progress hint (`2/5`, mint when complete) on the canvas and in Kanban; the detail panel shows a thin progress bar.
2. **Attachments.** _Attach files_ in the detail panel (file picker; drag-and-drop works where the webview passes files through) copies each file into `<app data>/attachments/<card id>/<attachment id>-<name>` (max 50 MB each). Images (PNG, JPEG, GIF, WebP, SVG, ≤ 15 MB) get a 40 px thumbnail; other files a document icon. Click opens the file in its default app (Preview for images and PDFs); _Show in Finder_ reveals it; _Remove_ deletes it. Cards show a paperclip count.
3. **Markdown notes.** The description is stored and edited as plain text and rendered in the detail panel with a small safe renderer (`src/lib/markdown.ts`): paragraphs, `#`–`###` headings, `-`/`*`/`1.` lists, `- [ ]` / `- [x]` task items (display only), `>` quotes, fenced code, `---`, **bold**, _italic_, ~~strike~~, `code`, `[links](https://…)` and bare URLs. The parser builds a tree rendered as React elements (no HTML injection); only `http(s)` and `mailto` links are clickable and open in the default browser. Click the preview to edit; leaving the field saves.
4. **Archive and Trash.** _Archive_ (detail panel header) hides a card from every view, count, search and reminder; the **Archive** view (sidebar footer) lists archived cards, projects and boards with _Restore_. _Delete_ (card action, detail panel) now moves a card to the **Trash** (same 5 s Undo toast); the Trash view lists cards with how long ago they were deleted, _Restore_, _Delete forever_ and _Empty Trash_ — both behind a confirm dialog. Cards trashed more than 30 days ago are purged at start-up and every 6 hours, with their attachment folders (orphaned folders are cleaned up too).
5. **Pinned cards.** Pin / unpin from the card's hover actions or the detail panel header. Pinned cards sort first (server order for every view; the List view keeps them first within each group) and show a small pin mark.
6. **Projects and boards management.** Right-click a project or personal board in the sidebar: _Rename_ (inline field), project color (the six project colors), _Archive_ (with Undo; restore from the Archive view) and _Delete_ (confirm). Renaming a project renames its board. Deleting a project moves its cards to the Trash and removes the project and its board; deleting a personal board moves the cards that live only on it to the Trash (project cards placed on it stay in their project). Archiving a project hides its cards from every view until it is restored.

## Data

Migration `0008_cards.sql`:

- `items.pinned` (0/1), `items.archived_at`, `items.deleted_at` (+ indexes); `projects.archived_at`, `boards.archived_at`.
- `subtasks(id, item_id → items ON DELETE CASCADE, title, done, position, created_at)`.
- `attachments(id, item_id → items ON DELETE CASCADE, name, file, mime, size, created_at)`; `file` is relative to `<app data>/attachments`.

`Item` gains `pinned`, `archivedAt`, `deletedAt`, `subtasksDone`, `subtasksTotal`, `attachmentCount`. `items::VISIBLE` (not archived, not trashed, not in an archived project) is applied to every list, count, search, project progress, reminder and AI related-card query; `items_get` still returns archived and trashed cards so the detail panel can show them with a restore banner. `projects_list` / `boards_list` skip archived places.

Commands (`commands/cards.rs`): `items_pin`, `items_archive`, `items_trash`, `items_restore`, `items_archived`, `items_trashed`, `items_delete_forever`, `trash_empty`, `subtasks_list|add|update|delete|reorder`, `attachments_list|add|read|open|delete`, `card_open_link`, `projects_rename|set_color|archive|delete|archived`, `boards_rename|archive|delete|archived`. All mutations emit `items:changed`.

## Decisions

- Archive and Trash are timestamps on the item, not separate tables: placements, links, subtasks and files survive a round trip, and restore is one `UPDATE`.
- Attachments are sent from the webview as base64 (works the same in the browser mock) and read back as raw bytes (`tauri::ipc::Response`) for thumbnails, so no asset-protocol scope or dialog plugin is needed. The 50 MB cap keeps the IPC payload reasonable.
- The Markdown renderer is a deliberately small subset written in-house: no dependency, no `dangerouslySetInnerHTML`, link schemes whitelisted.
- Deleting a project or board never deletes cards outright; it routes them through the Trash.
- Destructive confirmations use an in-app dialog (`ConfirmDialog`), not `window.confirm`, which WKWebView does not reliably show.
- Daily backups snapshot the database only; attachment files are not in the snapshot. The export includes archived cards (not the Trash).

## Changelog

- 2026-11-11 — First version: checklist, attachments, Markdown notes, Archive and Trash with 30-day purge, pinned cards, project and board management from the sidebar.
