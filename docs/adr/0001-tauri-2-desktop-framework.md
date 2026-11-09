# 0001 — Tauri 2 as the desktop framework

**Status:** Accepted · **Date:** 2026-11-09

## Context

NOMA is a desktop companion, not a document window: it needs several small transparent always-on-top windows (floating orb, voice overlay, reminder toasts), a menu-bar/tray item, global keyboard shortcuts, and native notifications. It also runs a 4B-parameter language model and a Whisper model on the same machine, so the shell's own memory footprint matters. The interface must feel instant (< 100 ms interactions, 60 fps motion) on an M2 with 16 GB.

## Decision

Build NOMA on **Tauri 2** with a Rust core and a React 19 / TypeScript frontend. The Rust core owns SQLite, the model sidecar supervisor, speech capture and the reminder scheduler; the frontend is a thin, design-faithful view layer talking to it through typed commands and events.

## Consequences

- Binary ~10–15 MB and a WebKit webview instead of a bundled Chromium; RAM left for the models.
- First-class plugins for windows, tray, global shortcuts, notifications, autostart and sidecars.
- Transparent windows on macOS require `macOSPrivateApi: true`, which excludes Mac App Store distribution. Accepted: distribution is a DMG.
- WebKit, not Chromium, renders the UI: CSS features are checked against Safari. Glass effects use `backdrop-filter`, supported.
- Rust is a second language in the repo; the data and AI layers are tested with `cargo test`.
- A Phase 0 spike (ADR-0002) verifies the floating-window requirements before Phase 1; if it fails, this decision is revisited in favor of Electron.

## Alternatives considered

- **Electron** — mature multi-window and tray support, Chromium everywhere. Rejected: ~150–200 MB footprint per app plus a Node runtime, next to models already using several GB; heavier idle CPU; slower cold start.
- **Native SwiftUI** — best macOS integration. Rejected: no path to Windows/Linux, and the design system (CSS tokens, handoff mockups) maps directly to the web.
- **Flutter** — single codebase, good performance. Rejected: desktop window-management story (transparent always-on-top auxiliary windows) is weaker, and the design handoff is CSS.
