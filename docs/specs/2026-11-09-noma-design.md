# NOMA — Design Document

**Status:** Approved (2026-11-09) · **Date:** 2026-11-09 · **Owner:** Alessandro Mastropasqua (`alemastro97`)

> YOUR SECOND MIND. — A personal AI desktop companion that listens, remembers, organizes and gently reminds.

This document is the source of truth a builder can plan from. Brand and visual rules live in [`design/`](../../design/) (the handoff: `tokens.css`, `noma-orb.css`, `screens/*.html`, `README.md`) and are **normative**: when this document and the design handoff disagree on a visual detail, the handoff wins.

---

## 1. Intent

NOMA removes the "where do I put this?" step from thinking. The user presses a shortcut, says or types something, and a correctly typed card (task, reminder, note, idea, project, event) appears on the right board with the right date and reminder. No task manager to open, no form to fill.

It must feel like a quiet, intelligent presence living on the desktop — a small violet orb — not an application window the user has to go to.

### Why now

Fast local models (3–4B parameters) are now good enough at structured extraction to run this flow privately, on-device, in under two seconds on an Apple Silicon laptop. That makes an always-listening-when-asked companion viable without cloud dependency or latency.

### Success looks like

- Capture → card on board in **< 3 s** end-to-end for a 5-second utterance on an M2 / 16 GB machine.
- UI interactions (open panel, switch view, drag card, expand orb) respond in **< 100 ms**; animations run at 60 fps.
- The app is used daily without the user ever thinking about _where_ something was saved.
- Visual fidelity: a side-by-side with `design/screens/*.html` shows no deviation in spacing, color, radius, type or motion.

## 2. Goals, non-goals, anti-goals

### Goals (v1)

1. **Capture anything** by voice (⌥ Space) or text (⌘⇧N) from anywhere on the desktop, and have AI turn it into a typed card.
2. **Spatial organization**: Canvas mode with free-positioned cards and connections is the signature view; List and Kanban are the practical views.
3. **Gentle reminders**: floating, minimal toast cards and OS notifications at the right time.
4. **Desktop presence**: floating always-on-top orb, tray/menu-bar item, compact translucent panel.
5. **Fully local by default**: data in a local SQLite database, STT and LLM on device. No account required.
6. **Premium feel**: pixel-faithful to the design system, calm motion, keyboard-first.
7. **Ship on macOS first**, with a build pipeline ready to add Windows and Linux.

### Non-goals (v1)

- Cloud sync, multi-device, collaboration, sharing.
- Calendar integrations (Google/Apple Calendar), email, third-party task tools.
- Calendar and Timeline views (view switcher shows them, marked "soon"; not designed yet).
- Text-to-speech / spoken replies from NOMA.
- Mobile apps.
- A general-purpose chat interface with the model.

### Anti-goals (would make it a failure even if it "works")

- Looking like a generic SaaS dashboard or an AI chatbot.
- Any perceptible lag: janky drag, slow view switches, an orb that stutters.
- Requiring the user to install or configure Ollama, Python, or models by hand.
- Aggressive or noisy: loud colors, constant motion, intrusive notifications.
- Sending user content to the network without an explicit opt-in.

## 3. Constraints

| Constraint      | Detail                                                                                                                                                                                                                                                                                                    |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Platform (v1)   | macOS 13+ on Apple Silicon and Intel. Windows 10+/Linux (AppImage/deb) in v2; the pipeline is prepared now.                                                                                                                                                                                               |
| Hardware target | M2, 16 GB RAM is the reference machine. LLM + STT together must stay under ~5 GB resident.                                                                                                                                                                                                                |
| Privacy         | On-device inference. Only outbound network calls in v1: downloading model files (with checksums) and GitHub release update checks.                                                                                                                                                                        |
| Assets          | Geist + Geist Mono bundled as woff2 (SIL OFL) — the app works offline, never loads Google Fonts. App icons rebuilt as SVG from `design/screens/Assets.html` and rendered to `.icns`/`.ico`/PNG with `tauri icon`. Tray icon is a monochrome N-symbol template image (not in handoff; builder derives it). |
| Design          | `design/` handoff is normative. Geist / Geist Mono, Lucide icons (stroke 1.75, 15–18 px), tokens from `tokens.css`, motion 150–300 ms with `cubic-bezier(.2,.8,.2,1)`, `prefers-reduced-motion` respected everywhere.                                                                                     |
| Accounts        | GitHub: personal account `alemastro97` (not the Orionis org).                                                                                                                                                                                                                                             |
| Process         | Spec-driven: every feature has a living spec in `docs/specs/`; decisions in `docs/adr/`; plans in `docs/plans/`. See §9.                                                                                                                                                                                  |
| Tests           | Lightweight. Unit tests for logic (extraction parsing, date resolution, stores, repositories), a handful of component tests, Rust tests for the data layer. No large E2E suite in v1; a smoke test that launches the app and captures a card is enough.                                                   |

## 4. Technical decisions (builder's calls, stated)

These were left to the builder ("scegli tu") and are recorded here as decisions. Each gets an ADR in `docs/adr/` when implemented.

### 4.1 Desktop framework: **Tauri 2** (Rust core + web frontend)

- Why not Electron: NOMA needs several small always-on-top transparent windows (orb, toast, overlay), a tray item and global shortcuts, and must feel instant. Tauri gives a ~10–15 MB binary, native WebKit on macOS, a Rust core for SQLite/STT/LLM supervision, and first-class multi-window, tray, global-shortcut and notification plugins. Memory footprint matters because the models already use several GB.
- Plugins: `window`, `tray-icon`, `global-shortcut`, `notification`, `shell` (sidecar), `fs`, `store` (settings), `autostart`. (`updater` deferred, see §4.8.)
- macOS specifics: `macOSPrivateApi: true` for transparent windows (rules out the Mac App Store — acceptable, distribution is a DMG). Overlay/orb/toast windows: transparent, always-on-top, `visibleOnAllWorkspaces`, shown over fullscreen apps (NSWindow level + collection behavior set from Rust). Glass is **CSS `backdrop-filter`** on the webview; native `window-vibrancy` only if CSS blur proves too costly.
- **Phase 0 spike (go/no-go):** a throwaway transparent always-on-top window over a fullscreen app on all Spaces, toggled by a global shortcut, with a 60 fps orb animation and idle CPU < 1 %. If any of these fails in Tauri, the fallback is Electron and this section is rewritten before Phase 1 starts.
- Rust toolchain: stable, updated to current (`rustup update`).

### 4.2 Frontend: **React 19 + TypeScript + Vite**

- Styling: **plain CSS with CSS Modules**, consuming `design/tokens.css` directly. No Tailwind — the handoff is already a token system and adding a utility layer would drift from it.
- Motion: **`motion`** (Framer Motion) for springs, layout animations and presence; plain CSS transitions for hover/focus micro-interactions. All durations and easing from tokens.
- Icons: `lucide-react`.
- State: **Zustand** stores (items, boards, UI, AI status), hydrated from the Rust backend through typed `invoke` commands and events.
- Canvas: **`@xyflow/react`** (React Flow) with fully custom nodes/edges. It gives performant pan/zoom, drag, selection and edges out of the box; every visual is ours (cards as nodes, dashed-flow connections as edges, 24 px dot grid background). If it fights the design, fall back to a custom transform-based canvas — the card components are independent of it.
- Routing: in-app view state only (no URL router); one main window + auxiliary windows each mounting a small React root.

### 4.3 Data: **SQLite via `rusqlite`** in the Rust core

- **Concepts.** An _item_ is any card (type: `task | reminder | note | idea | project_ref | event`). A _project_ is a container with a color; every project owns one _board_. A _board_ is a spatial canvas; users can also create personal boards not tied to a project. Inbox, Today, Upcoming, Ideas and Notes are **filters** over items, not boards. An item lives on at most one board in v1 (its project's board, a personal board, or none = Inbox). Status enum: `inbox | next | doing | done` (`done` ⇔ `completed_at` set); Kanban columns are the statuses; the handoff's "completed" card state is `done`.
- Schema (v1): `items` (id, type, title, body, status, priority, due_at, remind_at, snoozed_until, project_id, created/updated/completed_at), `projects` (id, name, color, board_id), `boards` (id, name, kind: project|personal), `placements` (item_id, board_id, x, y — separate table so multi-board placement is a v2 data change, not a migration rewrite), `connections` (from_item, to_item, label), `tags` + `item_tags`, `captures` (id, source: voice|text, raw_text, model_output_json, status: pending|processing|done|failed, latency_ms, item_id), `settings` (key/value).
- **Recurrence (v1):** `items.recurrence` holds an RFC 5545 RRULE string limited to a supported subset — `FREQ=DAILY|WEEKLY|MONTHLY|YEARLY`, `INTERVAL`, `BYDAY` (weekly), `BYMONTHDAY` (monthly), optional `UNTIL`/`COUNT`. Completing a recurring item does not set `done`; it logs the occurrence in `item_occurrences` (item_id, scheduled_at, completed_at, skipped) and advances `due_at`/`remind_at` to the next occurrence computed in Rust (`rrule` crate). The card shows a small repeat glyph and the rule in words ("every Monday").
- Location: Tauri app-data dir (`~/Library/Application Support/noma/noma.db`). Backup = copy of the DB file from Settings; export = JSON; import is v2.
- Migrations as numbered SQL files applied at startup.
- The frontend never touches SQL; it calls commands (`items.list`, `items.create`, `items.update`, `items.move`, `connections.create`, …) and listens to change events.
- ⌘K is a **command palette + search**: top section actions and navigation (views, boards, "new task", "pause reminders"…), below it FTS5 results over items.

### 4.4 AI runtime: **llama.cpp `llama-server` as a Tauri sidecar**, behind a provider abstraction

- The app downloads a platform-matched `llama-server` build and the model file on first run (checksum-verified, resumable, with progress in the onboarding flow). Nothing to install by hand.
- Talks to it over its OpenAI-compatible HTTP API on localhost with **JSON-schema constrained decoding** (GBNF) — this, more than model choice, is what makes extraction reliable.
- `LlmProvider` trait in Rust: `LlamaServerProvider` (default), `OllamaProvider` (developer convenience; used in dev when Ollama is running, never required), room for a cloud provider later behind an explicit opt-in.
- The server is started lazily on first capture and kept warm; idle unload after a configurable period to free memory.

### 4.5 Model: **Qwen3-4B-Instruct-2507, Q4_K_M** (~2.5 GB)

- Why: best small-model structured/JSON output in current reports, strong Italian and English, ~30–40 tok/s on M2 — extraction output is ~80 tokens, so well under a second after prompt processing.
- The **Instruct-2507** variant is non-thinking; the hybrid `Qwen3-4B` is not used (thinking would blow the latency budget). If a newer Qwen3.x 4B instruct lands, it is a manifest change plus an eval run.
- Hosting: official GGUF repos on Hugging Face (model) and llama.cpp GitHub Releases (`llama-server`), pinned by URL + SHA-256 in `models.json`. No own mirror in v1.
- Alternatives kept configurable in settings: Qwen3 1.7B (low-RAM), Gemma 3 4B.
- **Validation gate (Phase 2, first task):** measure warm extraction latency and eval accuracy on the reference machine. Pass = p95 < 1.2 s and ≥ 90 % correct `type`, ≥ 85 % correct `due_at` on the eval set. Fail → drop to 1.7B or change quantization before building on it.
- **Memory policy:** whisper model loads when the voice overlay opens and unloads 60 s after; `llama-server` starts on first capture and stays warm 10 min (configurable), then exits. Both resident together only during a voice capture (~4.5–5.5 GB peak including KV cache and Metal buffers). On machines with < 12 GB RAM the app selects whisper `small` and warns. 8 GB machines are supported but not a target.
- A tiny eval set (`ai/evals/*.jsonl`, ~40 Italian + English utterances → expected card JSON) is part of the repo and runs locally on demand; it guards model/prompt changes.

### 4.6 Speech-to-text: **whisper.cpp via `whisper-rs`**, in-process, Metal-accelerated

- Model: **large-v3-turbo** quantized (~800 MB – 1.6 GB) for Italian + English; `small` as a low-RAM option. Language auto-detected per utterance.
- Audio captured in Rust (`cpal`), with simple energy-based VAD; partial transcripts streamed to the overlay so the user sees words appear while speaking.
- Alternatives considered: Parakeet v3 (faster, weaker language coverage evidence for Italian), Moonshine (English-first). Whisper is the safe multilingual choice; the STT is behind a `SttProvider` trait so Parakeet can be trialled later.

### 4.7 Extraction contract

Input: transcript or typed text, current date/time, timezone, user locale, list of existing projects and tags, current board.
Output (JSON schema, constrained): `{ items: [{ type, title, body?, due_at?, remind_at?, recurrence?, priority?, project?, tags[], date_phrase?, confidence }] }` — `recurrence` is an RRULE string within the supported subset (validated in Rust; invalid → dropped with a "check repeat" chip) — 1 to 3 items per utterance (one is the normal case; "ricordami il latte e chiama Marco domani" yields two).

- Date/time phrases ("domani alle 9", "next Friday") are resolved **by the model** to ISO strings, given the current local datetime in the prompt. Rust does not re-parse natural language; it **validates** the ISO value (parses, is within [now − 1 day, now + 2 years], time plausible) and keeps `date_phrase` on the card. Invalid or implausible → `due_at` dropped and the card shows a "check date" chip with the phrase.
- Project matching: model proposes a name; Rust fuzzy-matches against existing projects (threshold), otherwise leaves it in Inbox with a suggestion chip.
- Low `confidence` → card still created in Inbox, with a subtle "check this" indicator. Never block the capture.
- **Capture lifecycle.** Every capture is a `captures` row. Text captured before the LLM is ready creates an Inbox card of type `note` with a "processing" shimmer and `captures.status = pending`; when the model becomes available, pending captures are processed and the card is re-typed in place (animated). Voice capture before STT is ready: the overlay says models are still downloading (with progress) and offers Tab-to-type. `failed` captures keep the raw text as a note and show a retry action.

### 4.8 Build, CI, releases

- GitHub Actions: `ci.yml` on PR/push — typecheck, lint, Vitest, `cargo test`, `cargo clippy`. `release.yml` on tag `v*` — `tauri-action` matrix: `macos-latest` (aarch64 + x86_64 → universal `.dmg`); `windows-latest` and `ubuntu-22.04` jobs present but `continue-on-error: true` until v2.
- v1 builds are **unsigned/un-notarized** (no Apple Developer certificate yet); first-run Gatekeeper note documented. Signing/notarization and the Tauri `updater` (needs its own keypair + hosted manifest) are v2 items, pending the distribution answer in §12.
- Downloaded `llama-server`: after checksum verification the app strips the quarantine attribute and ad-hoc `codesign`s the binary so Gatekeeper lets it run.
- Conventional Commits; `CHANGELOG.md` generated from them.

## 5. Product surface (v1)

### 5.1 Main window

Layout per `design/screens/Workspace.html`.

- **Sidebar** (collapsible, ⌘\): logo + orb, Inbox, Today, Upcoming, Projects (expandable), Ideas, Notes, Boards, Settings. Counts in Geist Mono.
- **Header**: current board/view title, search (⌘K), quick capture button, AI status (orb state at 20 px + label), profile/settings.
- **View switcher**: Canvas · Kanban · List · Calendar (soon) · Timeline (soon).
- **Canvas mode**: dot grid + faint violet glow, free-positioned cards, dashed-flow connections, pan (space-drag / trackpad), zoom (⌘ +/−, pinch), multi-select, snap-to-grid toggle. New cards fade+scale in from .97 over 280 ms.
- **List mode**: grouped by day (Today view) with checkboxes, per `design/screens/Light.html`.
- **Kanban mode**: columns by status (Inbox / Next / Doing / Done) or by project; same card shell. Not in the handoff — derived from the card and list design, kept plain.
- **Empty states**: copy from handoff, orb as companion.

### 5.2 Cards

One shell, six types, per `design/screens/Cards.html`. Type is signalled by colored icon + tinted border, never full fills. States: default, hover (lift 2 px / 200 ms), selected, completed (mint). Contextual actions appear on hover: complete, snooze, open, connect, delete. Inline edit of title on double-click; a detail panel (slide-over) for body, dates, project, tags.

### 5.3 Desktop presence

Per `design/screens/Desktop.html`.

- **Floating orb**: small transparent always-on-top window, draggable, remembers position, click → compact translucent panel (recent captures, today count, mic button). Hidden when the main window is focused (configurable).
- **Voice capture overlay** (⌥ Space, global): centered glass panel with the orb in LISTENING state, audio-reactive ring, live transcript; Esc cancel, ↵ confirm early, Tab to switch to typing. After stop: PROCESSING → SUCCESS pulse → card toast → overlay closes.
- **Quick capture** (⌘⇧N, global): small glass input; same extraction pipeline.
- **Reminder toasts**: glass card top-right, amber indicator, snooze (10 min · 1 h · tomorrow 9:00) / complete (for recurring: "done for today" advances to the next occurrence); also a native notification when the app is in the background. **Scheduler** runs in the Rust core (tokio timer re-armed on DB change and on system wake); fires while the main window is closed. Reminders missed during sleep/quit fire on next launch/wake as a single grouped "while you were away" toast.
- **App lifecycle**: menu-bar agent. Closing the main window keeps NOMA running (orb, tray, scheduler); Dock icon shown only while the main window is open (activation policy switches to accessory otherwise). Quit only from tray or ⌘Q.
- **Tray / menu bar**: monochrome template icon (N symbol) with a small state dot, menu: Capture, Open NOMA, Pause reminders, Quit.

### 5.4 Settings (minimal)

Theme (dark/light/system), shortcuts, models (installed, download/remove, choose), reminders (quiet hours), start at login, data location + export (JSON). Not designed — follow the list design and spacing.

### 5.5 Onboarding (first run)

Three steps, orb-led: welcome → download models (progress, sizes shown up front: ~4 GB total) → grant microphone permission and try a capture. Skippable; text capture works before models finish (queued).

## 6. Motion & states (summary)

- NomaOrb states and timings from `design/noma-orb.css`; it is one component (`<NomaOrb state size />`) used at 96 → 20 px.
- All transitions 150–300 ms, token easing. Spring drag (stiffness ~500, damping ~40) in Canvas. Reduced motion: instant state changes, no floating/pulse.

## 7. Performance budget

- Cold start to interactive main window: < 1 s (models load lazily).
- Canvas with 500 cards: 60 fps pan/zoom; cards virtualized outside viewport.
- Capture pipeline: STT < 1 s for 5 s audio; LLM extraction < 1 s; DB write + UI < 50 ms.
- Idle: < 1 % CPU, orb animation GPU-composited; models unloaded after idle timeout.

## 8. Testing approach

Deliberately light.

- **Frontend (Vitest + Testing Library):** stores, extraction response → card mapping, date formatting, NomaOrb renders each state, card renders each type, view switcher.
- **Rust (`cargo test`):** repositories and migrations against an in-memory DB; date normalization; project fuzzy match; extraction JSON validation.
- **AI eval (manual/on demand):** `ai/evals` against the configured model; reports accuracy per field.
- **Smoke:** a script that builds, launches, creates a card via the text capture command, and asserts it is in the DB.

## 9. Working process: spec-driven

- `docs/specs/` — this design plus one living spec per feature (`<feature>.md` with Status: Draft / In progress / Shipped, Scope, UX, Data, Open questions, Changelog). Updated in the same PR as the code.
- `docs/adr/NNNN-<decision>.md` — architecture decisions (framework, AI runtime, model, STT, canvas lib, …).
- `docs/plans/` — implementation plans (superpowers:writing-plans output).
- `docs/specs/README.md` — index with status of every spec, so the state of the project is readable in one page.

## 10. Phasing

- **Phase 0 — Foundation:** repo, Tauri+React scaffold, tokens, NomaOrb, app shell (sidebar/header), SQLite + commands, CI.
- **Phase 1 — Core:** cards, Canvas, List, Inbox/Today/Upcoming/Projects/Ideas/Notes, ⌘K search, detail panel.
- **Phase 2 — AI capture:** sidecar manager + model download, extraction with schema, quick capture (text), eval set.
- **Phase 3 — Voice & desktop:** whisper STT, voice overlay, floating orb window, tray, reminder toasts + notifications, global shortcuts, onboarding.
- **Phase 4 — Polish & release:** Kanban, settings, light theme pass, performance pass, macOS release pipeline, README.
- **v2 (later):** Calendar, Timeline, Windows/Linux, signing/notarization, AI-suggested connections, semantic search (embeddings), cloud provider opt-in.

## 11. Left to the builder

- Exact component decomposition and file layout.
- Kanban and Settings visual details (derive from handoff; keep plain).
- Prompt wording and schema details for extraction; VAD thresholds.
- Choice of quantization/revision within the pinned families, validated by the eval set.
- Virtualization strategy for the canvas.

## 12. Owner decisions (2026-11-09)

1. **Repository**: `alemastro97/noma`, private.
2. **UI language**: English only. Voice and text input understand Italian and English. No i18n layer in v1 (strings kept in one module so it can be added later).
3. **Distribution of v1**: personal use. Unsigned DMG; signing, notarization and auto-update are v2.
4. **Recurring reminders**: **in v1.** See §4.3 and §4.7.
