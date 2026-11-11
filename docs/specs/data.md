# Your data

**Status:** Shipped (Unreleased) · **Phase:** 4

Extends Settings → _Your data_ (backups and export, see desktop-presence.md §11) with restore, imports, a folder mirror, ways for other apps to reach NOMA, capture history and personal stats. Everything stays on this Mac. Code: `src-tauri/src/data/*`, `src-tauri/src/bin/noma-mcp.rs`, `src/views/settings/DataToolsSection.tsx`, `src/api/data.ts`.

## Scope

1. **Restore from backup.** Lists the snapshots in `<app data>/backups` (date, size, newest first) with _Restore_ and an inline confirmation. The snapshot is validated (`PRAGMA quick_check`, NOMA's `items` table present), copied next to the database (`noma.db.restoring`), a fresh snapshot of the current state is taken (so the restore can itself be undone), then the live connection is closed (checkpointing the WAL), stale `-wal`/`-shm` files are removed, the copy is renamed over `noma.db` and reopened (migrations run, so older snapshots are upgraded). If the swap fails the original file is reopened. Emits `items:changed` and `data:restored`; the main window reloads.
2. **Import.** Into the Inbox or a chosen project, without AI processing, at most 2000 cards per import; a summary says how many were created.
   - _Markdown / plain text_ (file picked in the UI, read in the webview): one card per list item (`-`, `*`, `+`, `1.`) or paragraph. `- [ ]` / `- [x]` items become tasks (done when checked); indented lines continue the previous item; headings and rules only separate blocks. Other items and paragraphs become notes. The title is the first line (≤ 120 chars); longer text is kept in the body.
   - _Todoist CSV export_: one task per `TYPE=task` row; `DESCRIPTION` goes to the body; priority 1 → urgent, 2 → high, else normal; a plain `YYYY-MM-DD` date becomes the due date (09:00 local), natural-language dates ("every monday") are kept in the body.
   - _Apple Notes_ (one note per card, from a chosen folder) and _Apple Reminders_ (open reminders of a chosen list; with a due date → reminder card with that reminder time, else task) are read through `osascript -l JavaScript`, read-only. macOS asks for Automation permission once (`NSAppleEventsUsageDescription`); a denial is reported as the `automation-denied` code and explained in the UI.
3. **Mirror to a folder.** Optional (`export.auto`), into a folder chosen with the native picker (`export.folder`): NOMA rewrites `<folder>/noma.md` (the same Markdown as _Export…_, from `backup.rs`) 5 s after the last `items:changed`, atomically (temp file + rename). Turning it on or choosing a folder writes it at once.
4. **URL scheme** (tauri-plugin-deep-link, scheme `noma` in `tauri.conf.json`; works in bundled builds): `noma://capture?text=…` creates a normal text capture (placeholder card + AI), `noma://open?item=<id>` shows the main window on that card (`ui:open-item`), `noma://today` shows Today (`ui:open-today`). Unknown links are ignored. Text is capped at 20 000 characters.
5. **CLI.** _Install command_ writes a POSIX `sh` script to `~/.local/bin/noma` (0755; the script is also shown and copyable): `noma <text>` or `echo … | noma` captures (`open -g noma://capture…`, URL-encoded with `osascript`), `noma open <id>`, `noma today`.
6. **MCP server.** `noma-mcp` (bundled next to the app executable) speaks MCP over stdio (JSON-RPC 2.0, one message per line) and opens NOMA's database directly (`$NOMA_DB` or `~/Library/Application Support/com.noma.app/noma.db`). Tools: `search_cards` (FTS, ≤ 50), `list_today`, `add_capture` (writes a placeholder card and a pending `captures` row; the running app's AI worker picks it up within ~30 s, or at next launch). Settings shows the binary path and a copyable `mcpServers` snippet.
7. **Capture history.** The latest captures (15, _Show more_ adds 30): raw text, voice/typed, time, status, AI latency and the resulting card's title; failed captures have _Retry_ (back to pending, worker nudged). Refreshes on `items:changed`.
8. **Personal stats** (_Your rhythm_): captures and completed cards per local week (Monday start) for the last 8 weeks as a small two-series bar chart, totals, and the average AI latency of successful captures in that window.

## Data

Migration `0012_data`: indexes `captures(created_at)` and `items(completed_at)`. Settings keys: `export.auto` (`"1"`/`"0"`), `export.folder` — written by `mirror_set`, not the UI whitelist. Commands: `backup_list`, `backup_restore`, `import_text`, `import_apple_sources`, `import_apple`, `pick_folder`, `mirror_get`, `mirror_set`, `cli_info`, `cli_install`, `mcp_info`, `captures_recent`, `captures_retry`, `stats_get`. Events: `data:restored`.

## Decisions

- Restore swaps the file under the single `AppState` connection mutex, so no command can run against a half-swapped database; the staged copy is made before the safety snapshot because pruning could delete the snapshot being restored.
- File imports are read by the webview (`<input type=file>`) and sent as text: no file-system plugin or extra permissions. Folder picking uses `osascript chooseFolder` for the same reason.
- Imports skip AI on purpose: they are bulk, already structured, and should be predictable.
- `add_capture` writes to the database instead of opening `noma://`, so it works when NOMA is closed and never steals focus; the cost is up to ~30 s before the card is sorted.
- Error strings meant for people live in `src/copy.ts`; the backend returns codes (`automation-denied`) where the UI needs to explain something.

## Changelog

- 2026-11-11 — created: restore, imports (Markdown/text, Todoist CSV, Apple Notes, Apple Reminders), folder mirror, `noma://` scheme, CLI, MCP server, capture history, personal stats.
