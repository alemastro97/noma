# 0002 — Floating windows on Tauri: go

**Status:** Accepted · **Date:** 2026-11-09

## Context

NOMA's desktop presence (floating orb, voice overlay, reminder toasts) needs small transparent windows that stay above everything, appear on every Space, show over other apps' fullscreen Spaces, toggle with a global shortcut and never steal focus. ADR-0001 chose Tauri 2 with the explicit condition that a Phase 0 spike proves this is possible; otherwise the fallback is Electron.

## Spike

A transparent, undecorated, always-on-top `orb` window (176 × 176, orb 52 px so the glow is not clipped) toggled by ⌥ Space through `tauri-plugin-global-shortcut`. Three iterations on macOS 26.5 (M2, 16 GB):

| Iteration | Approach                                                                                                                                                                                                                               | Result                                                                                                                                                          |
| --------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1         | Plain `NSWindow`, `collectionBehavior = CanJoinAllSpaces \| FullScreenAuxiliary \| Stationary`, status level, applied from the shortcut handler thread                                                                                 | Orb only on the app's own Space; glow clipped by the 72 px window. Cause: AppKit calls off the main thread are ignored.                                         |
| 2         | Same, applied via `run_on_main_thread`; window 176 px                                                                                                                                                                                  | Visible on all Spaces, glow intact, draggable, idempotent toggle. **Not** visible over another app's fullscreen Space. Same with `ActivationPolicy::Accessory`. |
| 3         | Window converted to a **non-activating `NSPanel`** (`tauri-nspanel` 2.1, `NSWindowStyleMask::NonactivatingPanel`, `CanJoinAllSpaces \| FullScreenAuxiliary \| Stationary \| IgnoresCycle`, level `Status + 1`, `orderFrontRegardless`) | **All checks pass**: over fullscreen Safari, on every Space, draggable, five rapid presses leave exactly one orb, Dock icon kept (regular activation policy).   |

Measurements (debug build, `top -pid`): idle with orb hidden 0.0–1.1 % CPU, 34 MB; orb visible and animating 0.0 % CPU over four 2 s samples (33 MB). Within the < 1 % average budget once the animation is GPU-composited (`will-change: transform` on the core); re-measure on a release build in Phase 4.

## Decision

**Go.** Tauri 2 stays. Every floating desktop element (orb, voice overlay, toasts, quick capture) is created as a Tauri window and converted to a non-activating `NSPanel` through `src-tauri/src/windows/macos.rs::configure_floating`, always on the main thread. Showing uses `orderFrontRegardless`, never `set_focus`, so the user's current app keeps focus. The main window remains a regular `NSWindow`.

## Consequences

- `tauri-nspanel` is a macOS-only dependency (gated by `cfg(target_os = "macos")`); Windows/Linux use Tauri's `alwaysOnTop` + `visibleOnAllWorkspaces` and will need their own spike in v2.
- A converted panel cannot become key window (`can_become_key_window: false`); the voice overlay in Phase 3 needs a second panel class with `can_become_key_window: true` so it can take keyboard input, shown with `show_and_make_key`.
- Panel operations must run on the main thread: all window helpers go through `AppHandle::run_on_main_thread`.
- Window sizes must include room for the orb's glow (`box-shadow` up to 1.4 em): ≥ 3.4 × orb size.

## Alternatives considered

- **Electron** — would work (BrowserWindow `setAlwaysOnTop(true, "screen-saver")` + `setVisibleOnAllWorkspaces(true, { visibleOnFullScreen: true })`), but the footprint argument from ADR-0001 stands and the spike removed the only blocker.
- **Accessory activation policy** — tested in iteration 2; does not by itself allow a regular window over fullscreen Spaces and would cost the Dock icon.
