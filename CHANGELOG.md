# Changelog

All notable changes to NOMA are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/) and versions follow [SemVer](https://semver.org/).

## Unreleased

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
