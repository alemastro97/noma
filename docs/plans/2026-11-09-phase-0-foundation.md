# NOMA Phase 0 — Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A running NOMA desktop app (macOS) with the design system, the NomaOrb, the app shell, a SQLite data layer exposed through typed commands, a go/no-go on Tauri's floating-window capabilities, and CI that tests and builds a DMG.

**Architecture:** Tauri 2 app: Rust core (`src-tauri/`) owns SQLite via `rusqlite` and exposes thin `#[tauri::command]`s; React 19 frontend (`src/`) consumes them through a typed `src/api/` layer into Zustand stores. Design tokens from `design/tokens.css` are the only source of color/radius/motion values. Each auxiliary window (orb, later overlay/toasts) is its own small React root under `src/windows/`.

**Tech Stack:** Tauri 2.x (stable, not the 3.0 alpha), Rust stable, React 19, TypeScript 5, Vite, Vitest + Testing Library, Zustand 5, `motion`, `lucide-react`, `rusqlite` (bundled SQLite, FTS5), `rrule`, pnpm 10, GitHub Actions + `tauri-apps/tauri-action`.

**Spec:** `docs/specs/2026-11-09-noma-design.md` (sections §3, §4.1–4.3, §4.8, §5.1, §6, §7, §9, §10 Phase 0).

## Global Constraints

- Tauri **2.x stable** (`tauri = "2"`, `@tauri-apps/cli` 2.x). Never the 3.0 alpha.
- Node 22, pnpm 10, Rust stable (run `rustup update stable` first; Tauri 2 needs ≥ 1.77).
- UI copy is **English only**; all user-facing strings live in `src/copy.ts`.
- Colors, radii, shadows, fonts and motion values come **only** from `src/styles/tokens.css` (verbatim copy of `design/tokens.css`). No hex literals in component CSS except where the handoff itself inlines one (document each).
- Motion: durations `--noma-fast` 150ms / `--noma-base` 200ms / `--noma-slow` 300ms, easing `--noma-ease: cubic-bezier(.2,.8,.2,1)`. `@media (prefers-reduced-motion: reduce)` disables every animation.
- Fonts: Geist and Geist Mono **bundled** as woff2; no remote font loading.
- Icons: `lucide-react`, `strokeWidth={1.75}`, sizes 15–18 px (14 px inside card "kind" labels, 12 px in chips).
- Small metadata text uses `--noma-text-2` (#A6A0B5), never `--noma-text-muted`.
- `macOSPrivateApi: true` (transparent windows; no Mac App Store).
- Commits: Conventional Commits. Git author = the user's personal account `alemastro97`. No AI attribution trailers.
- Every task that changes behavior updates the relevant living spec in `docs/specs/` in the same commit.
- Performance budget: cold start to interactive main window < 1 s; idle CPU < 1 % with the orb animating.

## Review Focus

Inputs the spec implies but no feature test exercises; each has a pinned test in the owning task.

1. **Reduced motion on**: orb and shell render with zero running animations (Task 4 test `respects prefers-reduced-motion`).
2. **Theme set to `system` while the OS theme flips**: `data-theme` follows the OS change without reload (Task 3 test `system theme tracks matchMedia change`).
3. **Database directory does not exist on first launch**: `open_db` creates parent directories and migrates (Task 7 test `open_creates_parent_dirs`).
4. **Completing a recurring item whose rule has ended (`COUNT`/`UNTIL` exhausted)**: item becomes `done` instead of advancing (Task 7 test `complete_recurring_exhausted_marks_done`).
5. **Global shortcut pressed twice quickly / orb window already visible**: toggle is idempotent, no duplicate windows, no panic (Task 6 manual check recorded in ADR-0002, plus Rust test `toggle_visibility_is_idempotent` on the pure toggle function).

---

### Task 1: Repository, GitHub remote and process scaffolding

**Files:**
- Create: `.gitignore`, `README.md`, `CHANGELOG.md`, `LICENSE` (MIT, copyright Alessandro Mastropasqua)
- Create: `docs/specs/README.md` (spec index), `docs/specs/foundation.md` (living spec for Phase 0, Status: In progress)
- Create: `docs/adr/README.md`, `docs/adr/0000-template.md`, `docs/adr/0001-tauri-2-desktop-framework.md`
- Existing: `design/**`, `docs/specs/2026-11-09-noma-design.md`, `docs/plans/2026-11-09-phase-0-foundation.md`

**Interfaces:**
- Produces: the repo at `~/Documents/side-projects/Noma` on branch `main`, remote `git@github.com:alemastro97/noma.git` (private). Spec index format: a table `| Spec | Status | Phase |`.

- [ ] **Step 1: Init git, set author to the personal account**

Run from the repo root: `git init -b main && git config user.name "Alessandro Mastropasqua" && git config user.email "<alemastro97 noreply email from `gh api user/emails` or the user's GitHub-visible email>"`. Verify `gh auth status` shows `alemastro97` active.

- [ ] **Step 2: Write `.gitignore`** covering Node (`node_modules`, `dist`), Rust/Tauri (`src-tauri/target`, `src-tauri/gen/schemas`), macOS (`.DS_Store`), editors, `.env*`, and `*.local`.

- [ ] **Step 3: Write `README.md`**: one-paragraph description (tagline "YOUR SECOND MIND."), stack line, `pnpm install && pnpm tauri dev` quick start (will be true after Task 2), links to `docs/specs/README.md` and `design/README.md`, Gatekeeper note for unsigned builds.

- [ ] **Step 4: Write the ADR template and ADR-0001**

`0000-template.md` sections: Title, Status (Proposed/Accepted/Superseded), Date, Context, Decision, Consequences, Alternatives considered. `0001-tauri-2-desktop-framework.md`: Status Accepted; Decision and Alternatives copied in substance from spec §4.1 (Electron rejected for footprint/memory next to multi-GB models).

- [ ] **Step 5: Write the spec index and the foundation spec**

`docs/specs/README.md` lists the design doc (Status: Approved) and `foundation.md` (Status: In progress, Phase 0). `foundation.md` has: Scope (the ten tasks of this plan in one line each), Data (pointer to §4.3), Open questions (empty), Changelog (dated entries, first: "2026-11-09 created").

- [ ] **Step 6: Create the private GitHub repo and push**

```bash
gh repo create alemastro97/noma --private --description "NOMA — your second mind. Personal AI desktop companion." --source . --remote origin
git add -A && git commit -m "docs: add design spec, phase 0 plan, design handoff and process scaffolding" && git push -u origin main
```
Expected: `gh repo view alemastro97/noma --json visibility` → `"PRIVATE"`.

---

### Task 2: Tauri 2 + React 19 scaffold with lint and test toolchain

**Files:**
- Create: `package.json`, `pnpm-lock.yaml`, `vite.config.ts`, `vitest.config.ts`, `tsconfig.json`, `tsconfig.node.json`, `eslint.config.js`, `.prettierrc`, `index.html`
- Create: `src/main.tsx`, `src/app/App.tsx`, `src/test/setup.ts`, `src/app/App.test.tsx`
- Create: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/build.rs`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`, `src-tauri/icons/*` (placeholder from the generator; replaced in Task 5)
- Create: `.nvmrc` (`22`), `rust-toolchain.toml` (`channel = "stable"`)

**Interfaces:**
- Produces: `pnpm dev | build | tauri | test | test:watch | lint | typecheck | format` scripts; `pnpm tauri dev` opens the main window `main` (1280×820, min 960×640, `titleBarStyle: "Overlay"`, `hiddenTitle: true`, background `#111116`); Rust crate `noma` with `lib.rs` exposing `pub fn run()`.

- [ ] **Step 1: Update Rust and scaffold in the scratchpad, then merge into the repo**

```bash
rustup update stable
cd "$SCRATCHPAD" && pnpm create tauri-app@latest noma --template react-ts --manager pnpm --yes
rsync -a --exclude node_modules --exclude .git "$SCRATCHPAD/noma/" ~/Documents/side-projects/Noma/
```
Then set in `src-tauri/tauri.conf.json`: `productName: "NOMA"`, `identifier: "com.noma.app"`, `app.macOSPrivateApi: true`, window `main` as in Interfaces, `bundle.targets: ["dmg", "app"]`, `bundle.category: "Productivity"`. Set `package.json` `name: "noma"`, `private: true`, `packageManager: "pnpm@10"`.

- [ ] **Step 2: Add the test and lint toolchain**

`pnpm add -D vitest @vitest/coverage-v8 jsdom @testing-library/react @testing-library/jest-dom @testing-library/user-event eslint @eslint/js typescript-eslint eslint-plugin-react-hooks eslint-plugin-react-refresh prettier`. `vitest.config.ts`: `environment: "jsdom"`, `setupFiles: ["src/test/setup.ts"]`, `globals: true`, `css: true`. `src/test/setup.ts` imports `@testing-library/jest-dom/vitest` and mocks `@tauri-apps/api/core` `invoke` with `vi.fn()` by default. `eslint.config.js`: flat config, typescript-eslint recommended + react-hooks + react-refresh. Scripts: `"test": "vitest run"`, `"test:watch": "vitest"`, `"lint": "eslint src"`, `"typecheck": "tsc --noEmit -p tsconfig.json"`, `"format": "prettier --write ."`.

- [ ] **Step 3: Write the failing smoke test**

```tsx
// src/app/App.test.tsx
import { render, screen } from "@testing-library/react";
import { App } from "./App";
test("renders the NOMA shell root", () => {
  render(<App />);
  expect(screen.getByRole("application", { name: "NOMA" })).toBeInTheDocument();
});
```

- [ ] **Step 4: Run it to verify it fails**

Run: `pnpm test` → Expected: FAIL (`App` not exported / role not found).

- [ ] **Step 5: Implement `App`** in `src/app/App.tsx` as `export function App(): JSX.Element` rendering `<div role="application" aria-label="NOMA" className={styles.app}>` (CSS module created in Task 3; for now a plain class). `src/main.tsx` mounts `<App />` in `StrictMode`. Delete the template's `App.css`/greeting code.

- [ ] **Step 6: Verify everything passes**

Run: `pnpm typecheck && pnpm lint && pnpm test` → all PASS. Run `cd src-tauri && cargo test` → `test result: ok. 0 passed`. Run `pnpm tauri dev` → window opens titled NOMA with dark background; close it.

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "feat: scaffold Tauri 2 + React 19 app with vitest and eslint"
```

---

### Task 3: Design tokens, bundled fonts, global styles and theme

**Files:**
- Create: `src/styles/tokens.css` (verbatim copy of `design/tokens.css`), `src/styles/global.css`, `src/styles/fonts.css`
- Create: `src/assets/fonts/Geist[wght].woff2`, `src/assets/fonts/GeistMono[wght].woff2` (copied from the `geist` npm package `dist/fonts/**`)
- Create: `src/theme/theme.ts`, `src/theme/theme.test.ts`, `src/theme/tokens.test.ts`
- Modify: `src/main.tsx` (import styles, call `initTheme()`)

**Interfaces:**
- Produces: `export type Theme = "dark" | "light" | "system"`; `export function applyTheme(theme: Theme, root?: HTMLElement): void` (sets `data-theme="dark"|"light"` on `document.documentElement`, resolving `system` via `matchMedia("(prefers-color-scheme: light)")`); `export function initTheme(): () => void` (reads saved theme from `localStorage["noma.theme"]` default `"system"`, applies it, subscribes to matchMedia changes while `system`, returns unsubscribe); `export function setTheme(theme: Theme): void` (persists + applies). CSS variables `--noma-*` available app-wide.

- [ ] **Step 1: Write the failing tests**

```ts
// src/theme/tokens.test.ts
import { readFileSync } from "node:fs";
test("tokens.css is identical to the design handoff", () => {
  expect(readFileSync("src/styles/tokens.css", "utf8")).toBe(readFileSync("design/tokens.css", "utf8"));
});
```
```ts
// src/theme/theme.test.ts — uses a matchMedia mock with a controllable `matches` and listener list
test("applyTheme('light') sets data-theme=light", …)
test("applyTheme('system') resolves from matchMedia", …)
test("system theme tracks matchMedia change", () => { /* initTheme(); flip mock matches → true; fire 'change'; expect data-theme 'light' */ })
test("setTheme persists to localStorage key noma.theme", …)
```

- [ ] **Step 2: Run to verify they fail** — `pnpm test src/theme` → FAIL (files missing).

- [ ] **Step 3: Add fonts and styles**

`pnpm add geist`, copy the variable woff2 files for Geist Sans and Geist Mono into `src/assets/fonts/`, write `fonts.css` with two `@font-face` (`font-display: swap`, `font-weight: 100 900`). `global.css`: box-sizing reset, `html,body,#root{height:100%}`, `body{margin:0;background:var(--noma-bg);color:var(--noma-text);font-family:var(--noma-font);font-size:14px;line-height:1.45;-webkit-font-smoothing:antialiased}`, `color-scheme: dark` / light under `[data-theme="light"]`, `:focus-visible{outline:2px solid var(--noma-lavender);outline-offset:2px}`, and a global reduced-motion rule `@media (prefers-reduced-motion: reduce){*,*::before,*::after{animation-duration:.01ms!important;animation-iteration-count:1!important;transition-duration:.01ms!important}}`. `pnpm remove geist` afterwards is allowed (fonts are vendored).

- [ ] **Step 4: Implement `src/theme/theme.ts`** per Interfaces.

- [ ] **Step 5: Verify** — `pnpm test src/theme` PASS; `pnpm tauri dev` shows Geist (check a glyph like the "a" tail) on `#111116`.

- [ ] **Step 6: Commit** — `git commit -m "feat: add design tokens, bundled Geist fonts, global styles and theme switching"`

---

### Task 4: NomaOrb component

**Files:**
- Create: `src/components/NomaOrb/NomaOrb.tsx`, `src/components/NomaOrb/noma-orb.css` (verbatim copy of `design/noma-orb.css`), `src/components/NomaOrb/useOrbState.ts`, `src/components/NomaOrb/NomaOrb.test.tsx`, `src/components/NomaOrb/useOrbState.test.ts`, `src/components/NomaOrb/index.ts`

**Interfaces:**
- Produces: `export type OrbState = "idle" | "listening" | "processing" | "success" | "error"`; `export function NomaOrb({ state = "idle", size = 56, label, className }: { state?: OrbState; size?: number; label?: string; className?: string }): JSX.Element` — renders the handoff markup (`div.noma-orb.s-<state>` with `font-size: <size>px`, `role="img"`, `aria-label={label ?? "NOMA is " + state}`); `export function useOrbState(initial?: OrbState): { state: OrbState; set: (s: OrbState) => void; flashSuccess: () => void; flashError: (ms?: number) => void }` — `flashSuccess` sets `success` then returns to `idle` after **1400 ms**; `flashError` sets `error` and returns to `idle` after `ms` (default 2500).

- [ ] **Step 1: Write the failing tests**

```tsx
// NomaOrb.test.tsx
test.each(["idle","listening","processing","success","error"] as const)("renders state class s-%s", (s) => { render(<NomaOrb state={s} />); expect(screen.getByRole("img")).toHaveClass(`s-${s}`); });
test("sizes via font-size", () => { render(<NomaOrb size={32} />); expect(screen.getByRole("img")).toHaveStyle({ fontSize: "32px" }); });
test("default aria-label", () => { render(<NomaOrb state="listening" />); expect(screen.getByRole("img", { name: "NOMA is listening" })).toBeInTheDocument(); });
test("respects prefers-reduced-motion", () => { /* read noma-orb.css text; expect it to contain "@media (prefers-reduced-motion:reduce)" and "animation:none!important" */ });
```
```ts
// useOrbState.test.ts (fake timers, renderHook)
test("flashSuccess returns to idle after 1400ms", …)   // advance 1399 → still success; 1400 → idle
test("flashError returns to idle after default 2500ms", …)
test("set() cancels a pending flash timer", …)
```

- [ ] **Step 2: Run to verify fail** — `pnpm test src/components/NomaOrb` → FAIL.

- [ ] **Step 3: Implement** `NomaOrb.tsx` (markup exactly as the comment block in `noma-orb.css`: three `.ring`, two `.orbit`, `.core`, `.err`) and `useOrbState.ts` with a single `useRef<number>` timer cleared on `set` and on unmount.

- [ ] **Step 4: Verify** — tests PASS. Temporarily render `<NomaOrb size={96} state="processing" />` in `App` under `pnpm tauri dev` and compare with `design/screens/NomaOrb.html` opened in a browser; remove it after.

- [ ] **Step 5: Commit** — `git commit -m "feat: add NomaOrb component with five states and timed success/error flashes"`

---

### Task 5: App shell — sidebar, header, view switcher, empty states, app icons

**Files:**
- Create: `src/copy.ts`
- Create: `src/stores/ui.ts`, `src/stores/ui.test.ts`
- Create: `src/components/Sidebar/Sidebar.tsx`, `Sidebar.module.css`, `Sidebar.test.tsx`, `NavItem.tsx`
- Create: `src/components/Header/Header.tsx`, `Header.module.css`, `Header.test.tsx`, `ViewSwitcher.tsx`, `AiStatusChip.tsx`
- Create: `src/components/EmptyState/EmptyState.tsx`, `EmptyState.module.css`
- Create: `src/components/Logo/NSymbol.tsx` (SVG from `design/README.md`, `stroke="currentColor"`, dot `var(--noma-lavender)`), `src/components/Logo/Wordmark.tsx`
- Create: `src/app/Workspace.tsx` (main area: renders `EmptyState` for the active target; canvas comes in Phase 1)
- Create: `src/hooks/useHotkey.ts`
- Modify: `src/app/App.tsx` (compose Sidebar + Header + Workspace), `src/app/App.module.css`
- Create: `src-tauri/icons/icon.svg` (dark app icon rebuilt from `design/screens/Assets.html`), regenerate `src-tauri/icons/*` via `pnpm tauri icon src-tauri/icons/icon.svg`

**Interfaces:**
- Produces (store `src/stores/ui.ts`, Zustand):
  ```ts
  export type NavTarget = { kind: "inbox" } | { kind: "today" } | { kind: "upcoming" } | { kind: "ideas" } | { kind: "notes" } | { kind: "settings" } | { kind: "project"; id: string } | { kind: "board"; id: string };
  export type ViewMode = "canvas" | "kanban" | "list" | "calendar" | "timeline";
  export const AVAILABLE_VIEW_MODES: ViewMode[] = ["canvas", "kanban", "list"];   // calendar/timeline shown disabled with copy.soon
  interface UiState { target: NavTarget; viewMode: ViewMode; sidebarCollapsed: boolean; setTarget(t: NavTarget): void; setViewMode(m: ViewMode): void; toggleSidebar(): void; }
  export const useUiStore = create<UiState>()(persist(..., { name: "noma.ui" }));
  ```
  `setViewMode` ignores modes not in `AVAILABLE_VIEW_MODES`.
- Produces `src/copy.ts` with `nav` labels (Inbox, Today, Upcoming, Ideas, Notes, Projects, Personal boards, Settings), `viewModes`, `soon: "Soon"`, `capture: "Capture"`, `search: "Search"`, `aiIdle: "Idle"`, `orbHere: "NOMA is here"`, `orbHint: "⌥ Space to talk"`, and `empty` map: `inbox: { title: "Your mind is clear. For now.", body: "Inbox is empty. Anything you tell NOMA lands here first." }`, `upcoming: { title: "Nothing demanding your attention.", body: "No reminders in the next 7 days." }`, `project: { title: "Every great thing starts somewhere.", body: "Empty project — press N to add the first card." }`, plus `today`, `ideas`, `notes`, `board` variants written in the same voice.
- Produces `useHotkey(combo: string, handler: () => void)` — combo like `"mod+\\"`, `"mod+k"`; attaches to `window` keydown, `mod` = ⌘ on macOS.
- Sidebar: `<Sidebar counts={Record<string, number>} projects={…} boards={…} />` — counts from the items store in Task 8 (empty object until then). Width 248 px expanded, 56 px collapsed (icons only, tooltips), transition `--noma-base` `--noma-ease`. ⌘\ toggles.
- Header: title with project color dot, Geist Mono count, `ViewSwitcher`, search box (visual, `⌘K` hint — opens nothing yet), Capture button (`--noma-lavender` bg, `#16112A` text as in handoff), `AiStatusChip` (`<NomaOrb size={16} />` + label), avatar button.

- [ ] **Step 1: Write failing tests**

```ts
// ui.test.ts
test("default target is inbox and view mode canvas", …)
test("setViewMode('calendar') is ignored", …)
test("toggleSidebar flips and persists to localStorage noma.ui", …)
```
```tsx
// Sidebar.test.tsx
test("clicking Today sets target today and marks it current (aria-current=page)", …)
test("renders project rows with color dot and count", …)
test("collapsed sidebar hides labels but keeps accessible names", …)
// Header.test.tsx
test("Calendar and Timeline buttons are disabled and labelled Soon", …)
test("active view mode has aria-pressed=true", …)
test("shows Idle AI status with an orb", …)
```

- [ ] **Step 2: Run to verify fail** — `pnpm test src/stores src/components/Sidebar src/components/Header` → FAIL.

- [ ] **Step 3: Implement** store, copy, hooks and components following `design/screens/Workspace.html` measurements: sidebar `padding: 16px 12px`, nav row `min-height 34px; padding 0 10px; border-radius 9px; gap 10px; font-size 14px`, active/hover bg `--noma-surface`, section label 12 px `--noma-text-2`, bottom orb card (`bg --noma-bg-2`, border `--noma-surface-elevated`, radius 12). Header `padding 14px 24px; border-bottom 1px solid var(--noma-surface)`; view switcher pill `padding 3px; radius 10; bg --noma-bg-2; border --noma-surface-elevated`, mode button `min-height 32px; padding 0 10px; radius 8; font-size 13px`, active bg `--noma-surface-elevated`. Workspace background: `radial-gradient(circle, var(--noma-surface-elevated) 1px, transparent 1.2px) 0 0/24px 24px` plus `radial-gradient(700px 500px at 60% 40%, rgba(109,74,255,.08), transparent 70%)`. Empty state card: centered, `padding 36px 28px; radius var(--noma-radius-panel)`, orb 44 px, title 17 px/500, body 13 px `--noma-text-2`.

- [ ] **Step 4: Rebuild app icons** — author `src-tauri/icons/icon.svg` (1024 viewBox: rounded square `#17171F`, N symbol stroke `#F4F1FA` width scaled, dot `#B9A3FF`, subtle violet radial glow behind per `Assets.html`), run `pnpm tauri icon src-tauri/icons/icon.svg`.

- [ ] **Step 5: Verify** — tests PASS; `pnpm tauri dev`: compare side-by-side with `design/screens/Workspace.html` (sidebar width, row heights, header, switcher). ⌘\ collapses/expands smoothly. Light theme: `setTheme("light")` in devtools matches `design/screens/Light.html` surfaces. Dock icon shows the new icon.

- [ ] **Step 6: Commit** — `git commit -m "feat: add app shell with sidebar, header, view switcher, empty states and app icon"`

---

### Task 6: Floating-window spike (go/no-go) → ADR-0002

**Files:**
- Create: `docs/adr/0002-floating-windows-on-tauri.md`
- Modify: `src-tauri/tauri.conf.json` (add window `orb`), `src-tauri/Cargo.toml` (add `tauri-plugin-global-shortcut`, `objc2`, `objc2-app-kit` on macOS), `src-tauri/src/lib.rs`
- Create: `src-tauri/src/windows/mod.rs`, `src-tauri/src/windows/orb.rs`, `src-tauri/src/windows/visibility.rs`
- Create: `src/windows/orb/index.html`, `src/windows/orb/main.tsx`, `src/windows/orb/OrbWindow.tsx`, `src/windows/orb/orb.module.css`
- Modify: `vite.config.ts` (multi-page: `main: index.html`, `orb: src/windows/orb/index.html`), `src-tauri/capabilities/default.json` (+ `orb` window, global-shortcut permissions)

**Interfaces:**
- Window `orb`: `width: 72, height: 72, transparent: true, decorations: false, alwaysOnTop: true, visibleOnAllWorkspaces: true, skipTaskbar: true, resizable: false, shadow: false, visible: false, url: "src/windows/orb/index.html"`.
- Rust: `pub fn configure_floating(window: &tauri::WebviewWindow)` — on macOS sets `NSWindowCollectionBehavior` = `canJoinAllSpaces | fullScreenAuxiliary | stationary` and level `NSFloatingWindowLevel + 1` so it shows over fullscreen apps; no-op elsewhere. `pub fn toggle_visibility(is_visible: bool) -> bool` in `visibility.rs` (pure: returns the next state) used by `pub fn toggle_orb(app: &tauri::AppHandle)`.
- Global shortcut `Alt+Space` registered at setup → `toggle_orb`. (Phase 3 rebinds it to the voice overlay; the orb spike reuses the key only to prove the mechanism.)
- `OrbWindow` renders `<NomaOrb size={56} />` centered on a transparent body (`background: transparent`, `-webkit-app-region: drag` on the root so the orb is draggable).

- [ ] **Step 1: Write the failing Rust test**

```rust
// src-tauri/src/windows/visibility.rs
#[cfg(test)] mod tests { use super::*;
  #[test] fn toggle_visibility_is_idempotent() { assert!(toggle_visibility(false)); assert!(!toggle_visibility(true)); assert_eq!(toggle_visibility(toggle_visibility(false)), false); } }
```
Run `cargo test` → FAIL (module missing).

- [ ] **Step 2: Implement** `visibility.rs`, `orb.rs`, `configure_floating`, the shortcut registration in `lib.rs` `setup`, the Vite multi-page config and the orb React root.

- [ ] **Step 3: Verify the go/no-go criteria and record them**

Run `pnpm tauri dev`; press ⌥Space: the orb appears in the top-right of the screen, transparent background, animating. Checks (each recorded with result in ADR-0002):
1. Visible over a **fullscreen** app (open Safari fullscreen, press ⌥Space).
2. Present on every Space (swipe between desktops).
3. Draggable; position survives hide/show.
4. 60 fps: Safari/WebKit no dropped frames visually; `top -pid $(pgrep -f NOMA | head -1) -l 3 -stats cpu` shows **< 1 % CPU** idle with the orb animating. If higher, set `will-change: transform` on `.core` and re-measure; record both numbers.
5. Toggling ⌥Space rapidly five times leaves exactly one orb window (`tauri::Manager::webview_windows().len() == 2`).
Decision: **Go** if 1, 2, 4 and 5 pass. Otherwise **No-go**: write the Electron fallback in the ADR and stop; Phase 1 must not start.

- [ ] **Step 4: Commit** — `git commit -m "feat: add floating orb window spike with global shortcut; record ADR-0002"`

---

### Task 7: SQLite data layer in Rust

**Files:**
- Modify: `src-tauri/Cargo.toml` (+ `rusqlite = { version = "0.3x", features = ["bundled", "chrono", "serde_json"] }`, `chrono`, `serde`, `serde_json`, `uuid` (v7), `rrule`, `thiserror`)
- Create: `src-tauri/src/db/mod.rs`, `src-tauri/src/db/migrations.rs`, `src-tauri/src/db/migrations/0001_init.sql`, `src-tauri/src/db/models.rs`, `src-tauri/src/db/items.rs`, `src-tauri/src/db/projects.rs`, `src-tauri/src/db/boards.rs`, `src-tauri/src/db/placements.rs`, `src-tauri/src/db/recurrence.rs`, `src-tauri/src/db/error.rs`
- Modify: `docs/specs/foundation.md` (Data section: final schema)

**Interfaces (all `pub`, in `noma::db`):**
```rust
pub fn open(path: &Path) -> Result<Connection, DbError>;          // creates parent dirs, WAL, foreign_keys=ON, runs migrations
pub fn open_in_memory() -> Result<Connection, DbError>;            // for tests
pub fn migrate(conn: &mut Connection) -> Result<u32, DbError>;     // returns applied version; idempotent

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)] #[serde(rename_all = "snake_case")]
pub enum ItemType { Task, Reminder, Note, Idea, ProjectRef, Event }
pub enum Status { Inbox, Next, Doing, Done }
pub enum Priority { Low, Normal, High, Urgent }

pub struct Item { pub id: String, pub kind: ItemType, pub title: String, pub body: Option<String>, pub status: Status, pub priority: Priority,
  pub due_at: Option<DateTime<Utc>>, pub remind_at: Option<DateTime<Utc>>, pub snoozed_until: Option<DateTime<Utc>>, pub recurrence: Option<String>,
  pub project_id: Option<String>, pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc>, pub completed_at: Option<DateTime<Utc>> }
pub struct NewItem { pub kind: ItemType, pub title: String, pub body: Option<String>, pub priority: Option<Priority>, pub due_at: Option<DateTime<Utc>>, pub remind_at: Option<DateTime<Utc>>, pub recurrence: Option<String>, pub project_id: Option<String>, pub board_id: Option<String> }
pub struct ItemPatch { /* every Item field except id/created_at as Option<Option<T>> for nullable ones; serde default */ }
pub enum ItemFilter { Inbox, Today { now: DateTime<Utc>, tz: Tz }, Upcoming { now: DateTime<Utc>, days: u32 }, Ideas, Notes, Project(String), Board(String), All }

pub fn create_item(conn: &Connection, input: NewItem, now: DateTime<Utc>) -> Result<Item, DbError>;   // id = uuid v7; status Inbox; priority default Normal; if board_id → placement at (0,0)
pub fn get_item(conn: &Connection, id: &str) -> Result<Option<Item>, DbError>;
pub fn list_items(conn: &Connection, filter: &ItemFilter) -> Result<Vec<Item>, DbError>;            // excludes Done except Filter::All; Today = due/remind within local day; Upcoming = within `days`
pub fn update_item(conn: &Connection, id: &str, patch: ItemPatch, now: DateTime<Utc>) -> Result<Item, DbError>;
pub fn delete_item(conn: &Connection, id: &str) -> Result<(), DbError>;
pub fn complete_item(conn: &Connection, id: &str, now: DateTime<Utc>) -> Result<Item, DbError>;     // recurring → advance via recurrence::next_after and insert item_occurrences; exhausted → Done
pub fn counts(conn: &Connection, now: DateTime<Utc>, tz: Tz) -> Result<Counts, DbError>;           // { inbox, today, upcoming, ideas, notes, by_project: HashMap<String,u32> }

pub struct Project { pub id: String, pub name: String, pub color: String, pub board_id: String, pub created_at: DateTime<Utc> }
pub fn create_project(conn, name: &str, color: &str, now) -> Result<Project, DbError>;              // also creates its board (kind project)
pub fn list_projects(conn) -> Result<Vec<Project>, DbError>;
pub struct Board { pub id: String, pub name: String, pub kind: BoardKind /* Project | Personal */ }
pub fn create_board(conn, name: &str, now) -> Result<Board, DbError>;  pub fn list_boards(conn) -> Result<Vec<Board>, DbError>;
pub struct Placement { pub item_id: String, pub board_id: String, pub x: f64, pub y: f64 }
pub fn place_item(conn, item_id, board_id, x, y) -> Result<Placement, DbError>;  pub fn list_placements(conn, board_id) -> Result<Vec<Placement>, DbError>;

// recurrence.rs
pub fn validate_rrule(rule: &str) -> Result<(), DbError>;                                         // subset: FREQ DAILY|WEEKLY|MONTHLY|YEARLY, INTERVAL, BYDAY, BYMONTHDAY, UNTIL, COUNT
pub fn next_after(rule: &str, dtstart: DateTime<Utc>, after: DateTime<Utc>) -> Result<Option<DateTime<Utc>>, DbError>;
```
Schema `0001_init.sql`: tables `schema_version`, `items`, `projects`, `boards`, `placements` (PK item_id, FK cascade), `connections` (id, from_item, to_item, label), `tags`, `item_tags`, `captures` (id, source, raw_text, model_output_json, status, latency_ms, item_id, created_at), `item_occurrences` (id, item_id, scheduled_at, completed_at, skipped), `settings` (key PK, value); indexes on `items(status)`, `items(due_at)`, `items(remind_at)`, `items(project_id)`; `items_fts` FTS5 (title, body, content=items) with insert/update/delete triggers. Timestamps stored as RFC 3339 text.

- [ ] **Step 1: Write failing tests** in each module's `#[cfg(test)]`:

`migrations`: `migrate_is_idempotent` (migrate twice → version 1 both times), `open_creates_parent_dirs` (tempdir + nested path).
`items`: `create_then_get_roundtrip`, `create_defaults_status_inbox_priority_normal`, `list_inbox_excludes_done_and_projects`, `list_today_uses_local_day` (tz `Europe/Rome`, due 23:30 local counts, next day 00:30 does not), `list_upcoming_within_days`, `update_patch_only_touches_given_fields`, `complete_sets_done_and_completed_at`, `complete_recurring_weekly_advances_due_and_logs_occurrence` (rule `FREQ=WEEKLY;BYDAY=MO`), `complete_recurring_exhausted_marks_done` (`FREQ=DAILY;COUNT=1`), `delete_removes_placement_cascade`, `fts_finds_by_body_word`.
`recurrence`: `validate_rejects_unsupported_freq` (`FREQ=HOURLY`), `next_after_weekly_monday`.
`projects/boards`: `create_project_creates_board`, `counts_by_project`.

- [ ] **Step 2: Run to verify fail** — `cargo test` → FAIL (modules missing).

- [ ] **Step 3: Implement** the modules per Interfaces. Use `uuid::Uuid::now_v7()`. Keep SQL in each repo module; map rows with one `fn row_to_item`.

- [ ] **Step 4: Verify** — `cargo test` → all PASS; `cargo clippy -- -D warnings` clean; `cargo fmt --check` clean.

- [ ] **Step 5: Update `docs/specs/foundation.md` Data section** with the final table list and the recurrence subset. Commit: `git commit -m "feat: add SQLite data layer with items, projects, boards, placements and recurrence"`

---

### Task 8: Tauri commands, typed TS API and items store

**Files:**
- Create: `src-tauri/src/state.rs` (`pub struct AppState { pub db: Mutex<Connection> }`), `src-tauri/src/commands/mod.rs`, `commands/items.rs`, `commands/projects.rs`, `commands/boards.rs`
- Modify: `src-tauri/src/lib.rs` (manage state, DB at `app.path().app_data_dir()?.join("noma.db")`, `invoke_handler`), `src-tauri/capabilities/default.json`
- Create: `src/api/types.ts`, `src/api/items.ts`, `src/api/projects.ts`, `src/api/boards.ts`, `src/api/events.ts`
- Create: `src/stores/items.ts`, `src/stores/items.test.ts`, `src/stores/projects.ts`
- Modify: `src/app/App.tsx` (load stores on mount, pass counts/projects/boards to `Sidebar`), `src/components/Sidebar/Sidebar.tsx` (consume stores directly instead of props)

**Interfaces:**
- Commands (names are the wire contract; all return `Result<T, String>`): `items_list(filter: ItemFilter) -> Vec<Item>`, `items_create(input: NewItem) -> Item`, `items_update(id: String, patch: ItemPatch) -> Item`, `items_delete(id: String)`, `items_complete(id: String) -> Item`, `items_counts() -> Counts`, `projects_list() -> Vec<Project>`, `projects_create(name: String, color: String) -> Project`, `boards_list() -> Vec<Board>`, `boards_create(name: String) -> Board`, `placements_list(boardId: String) -> Vec<Placement>`, `placements_set(itemId: String, boardId: String, x: f64, y: f64) -> Placement`. `now`/`tz` are taken in Rust (`Utc::now()`, `iana_time_zone`). After every mutation the command emits event `"items:changed"` with payload `{ ids: string[] }`.
- `src/api/types.ts` mirrors the Rust types with camelCase (`#[serde(rename_all = "camelCase")]` on all Rust structs). `ItemFilter` TS union: `{ kind: "inbox" } | { kind: "today" } | { kind: "upcoming"; days: number } | { kind: "ideas" } | { kind: "notes" } | { kind: "project"; id: string } | { kind: "board"; id: string } | { kind: "all" }` (serde adjacently/internally tagged with `tag = "kind"`).
- `src/api/items.ts`: `listItems(filter)`, `createItem(input)`, `updateItem(id, patch)`, `deleteItem(id)`, `completeItem(id)`, `getCounts()` — each a typed `invoke` wrapper. `src/api/events.ts`: `onItemsChanged(cb: (ids: string[]) => void): Promise<UnlistenFn>`.
- `src/stores/items.ts` (Zustand): `{ items: Record<string, Item>; counts: Counts; loading: boolean; load(filter: ItemFilter): Promise<void>; create(input): Promise<Item>; update(id, patch): Promise<void>; complete(id): Promise<void>; remove(id): Promise<void>; refreshCounts(): Promise<void>; subscribe(): Promise<UnlistenFn> }` with optimistic update on `complete`/`update` and rollback on error.

- [ ] **Step 1: Write failing store tests** (`invoke` mocked in `src/test/setup.ts`):

`load('inbox') stores items by id and clears loading`, `create calls items_create and inserts result`, `complete is optimistic then reconciled`, `complete rolls back when invoke rejects`, `refreshCounts updates counts`.

- [ ] **Step 2: Run to verify fail** — `pnpm test src/stores/items.test.ts` → FAIL.

- [ ] **Step 3: Implement** Rust commands (thin: lock state, call `db::*`, map error to `String`, emit event), register in `lib.rs`; TS api + store; wire `Sidebar` counts and project rows to the stores; `App` calls `load`, `refreshCounts`, `subscribe` in an effect.

- [ ] **Step 4: Verify** — `pnpm test` and `cargo test` PASS. `pnpm tauri dev`: in devtools run `await window.__TAURI__.core.invoke("items_create", { input: { kind: "task", title: "Hello" } })` → Inbox count becomes 1 within 100 ms without reload. Check `~/Library/Application Support/com.noma.app/noma.db` exists.

- [ ] **Step 5: Commit** — `git commit -m "feat: expose items, projects and boards commands with typed API and stores"`

---

### Task 9: CI and release pipelines

**Files:**
- Create: `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/dependabot.yml`
- Modify: `README.md` (badges, release instructions), `docs/specs/foundation.md`

**Interfaces:**
- `ci.yml`: on `push` to `main` and `pull_request`; `macos-latest`; steps: checkout, `pnpm/action-setup@v4`, `actions/setup-node@v4` (22, cache pnpm), `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `pnpm install --frozen-lockfile`, `pnpm typecheck`, `pnpm lint`, `pnpm test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (all in `src-tauri`).
- `release.yml`: on `push` tags `v*`; matrix `include`: `{ platform: macos-latest, args: "--target universal-apple-darwin" }`, `{ platform: windows-latest, args: "" , experimental: true }`, `{ platform: ubuntu-22.04, args: "", experimental: true }`; `continue-on-error: ${{ matrix.experimental == true }}`; macOS adds `rustup target add aarch64-apple-darwin x86_64-apple-darwin`; ubuntu installs `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`; uses `tauri-apps/tauri-action@v0` with `tagName: ${{ github.ref_name }}`, `releaseName: "NOMA ${{ github.ref_name }}"`, `releaseDraft: true`, `prerelease: true`, `args: ${{ matrix.args }}`; `GITHUB_TOKEN` only (no signing secrets yet).

- [ ] **Step 1: Write both workflows and dependabot** (`npm` + `cargo` + `github-actions`, weekly).

- [ ] **Step 2: Verify CI** — push `main`; `gh run watch` until green. Fix anything red (frozen lockfile, clippy) in-place.

- [ ] **Step 3: Commit** — `git commit -m "ci: add test workflow and multi-platform release workflow"`

---

### Task 10: First release dry run and Phase 0 close-out

**Files:**
- Modify: `CHANGELOG.md`, `docs/specs/README.md`, `docs/specs/foundation.md` (Status: Shipped), `src-tauri/tauri.conf.json` + `package.json` + `Cargo.toml` (`version: "0.0.1"`)

- [ ] **Step 1: Bump versions to 0.0.1, write CHANGELOG entry `## 0.0.1 — Foundation`** listing Tasks 2–9 in one line each.

- [ ] **Step 2: Tag and watch the release**

```bash
git commit -am "chore: release 0.0.1" && git tag v0.0.1 && git push && git push --tags
gh run watch
```
Expected: macOS job green, draft release `NOMA v0.0.1` with `NOMA_0.0.1_universal.dmg`; Windows/Linux may be red (allowed). Download the DMG, `xattr -dr com.apple.quarantine NOMA.app`, launch: main window + ⌥Space orb work from the bundled build.

- [ ] **Step 3: Mark `foundation.md` Shipped, update the index, commit** — `git commit -m "docs: mark foundation spec shipped"` and push.

---

## Self-review notes

- Spec coverage for Phase 0 (§10): repo/process ✔ T1, scaffold ✔ T2, tokens ✔ T3, NomaOrb ✔ T4, app shell ✔ T5, spike ✔ T6, SQLite + commands ✔ T7–T8, CI ✔ T9–T10. Light theme ✔ T3/T5. Recurrence data model (owner decision) ✔ T7. Kanban/List/Canvas rendering, cards, ⌘K palette, capture, AI, voice, tray, reminders, settings and onboarding are Phases 1–4 and get their own plans.
- Types used across tasks: `OrbState`/`NomaOrb` (T4 → T5, T6), `NavTarget`/`ViewMode`/`useUiStore` (T5 → T8), `Item`/`ItemFilter`/`Counts` Rust (T7) ↔ TS camelCase (T8), command names (T8 ↔ release smoke in T10).
- Review Focus items each have a pinned test: 1 → T4, 2 → T3, 3 → T7, 4 → T7, 5 → T6.
