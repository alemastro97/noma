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

See design document §4.3. Final schema is recorded here when Task 7 lands.

## Open questions

None.

## Changelog

- 2026-11-09 — created
