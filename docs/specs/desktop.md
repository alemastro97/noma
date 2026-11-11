# Desktop presence extras

**Status:** Shipped (Unreleased) · builds on [desktop-presence.md](desktop-presence.md)

## Scope

1. **Menu-bar count.** The tray item's title shows how many open things are left on Today (the same list as the orb panel's "n left"); nothing when 0. Settings → On your desktop → _Count in the menu bar_ (`ui.menubarCount`, on by default).
2. **Orb attention.** The resting orb glows softly in the reminder color (`--noma-reminder`) when an open item's reminder time (snooze first) is within the next 15 minutes or overdue by at most 24 hours. Voice feedback (listening, success, error) takes precedence; the tooltip says _Something is due soon_.
3. **Orb position per screen.** When a drag ends (no move for 500 ms) the orb's position is saved relative to the screen it is on (`orb.pos.<screen>` in logical points, `orb.lastScreen`). Whenever the orb is shown it goes back to its spot on the last screen if that screen is still connected, otherwise to its spot (or current place) on the current screen — always clamped so the orb itself stays on-screen (its transparent margin may overhang).
4. **Orb panel: search and capture.** The panel gains a full-text search field (same FTS search as the palette, 7 results, debounced; results replace Today while typing, a click opens the card in the main window; Esc clears, a second Esc closes) and a one-line capture field (↵ hands the text to NOMA in Auto mode, like quick capture). The panel is now a key panel so it can take typing; it is 540 pt tall.
5. **Quiet hours.** Settings → On your desktop → _Quiet hours_ with a daily _From_/_To_ window (`reminders.quietEnabled`, `reminders.quietFrom`, `reminders.quietTo`, `HH:MM`, default 22:00–08:00, may wrap past midnight). Inside it the reminder scheduler holds like a pause (no toasts, no notifications, no morning brief); when it ends, reminders that came due more than 10 min earlier are grouped into the existing "While you were away" toast. The orb stays still and never glows. macOS Focus is **not** read (see Decisions).
6. **Orb size and style.** Settings → On your desktop: size **S / M / L** (40 / 52 / 68 px; `orb.size`) and style **Lively** (default, the handoff animation), **Calm** (the resting orb doesn't float or spin) or **Faded** (45 % opacity until pointed at) (`orb.style`). Applied live through `presence:changed`.
7. **More global shortcuts.** In the existing rebindable system: _Open Today_ (main window on Today), _Search_ (main window with the command palette), _Open last capture_ (main window on the card of the most recent capture). All **unset by default** (`""`), since any obvious combination clashes with other apps; the recorder shows _Not set_ and _Clear_ instead of _Reset_. Stored as `shortcut.today`, `shortcut.search`, `shortcut.lastCapture`.

## Data

No migration: everything lives in the `settings` table (keys above, written through the whitelisted `settings_set`). New commands: `presence_get`, `presence_open_item`. Events: `presence:changed` (`{ attention, quiet, todayLeft, orbSize, orbStyle }`), `ui:open-palette`; the shortcuts reuse `ui:open-today` and `ui:open-item`. `shortcuts:changed` / `shortcuts_get` now carry `today`, `search`, `lastCapture`.

## Decisions

- One small loop (`presence.rs`) recomputes the state every 30 s, on every `items:changed` and on every UI setting change, and emits only when it differs; the tray title is updated from the same place.
- The attention window looks back at most 24 h so a forgotten old reminder doesn't make the orb glow forever.
- Quiet hours reuse the pause path of the scheduler instead of a separate queue: the reminders are simply not fired yet, so nothing can be lost, and the existing "missed" grouping produces the catch-up toast.
- **macOS Focus is skipped.** The only public API (`INFocusStatusCenter`) reports Focus only to apps with the Communication Notifications entitlement and a user permission prompt, and returns a bare on/off without the schedule; reading the Focus database directly is private. Native notifications already respect Focus on their own.
- The orb's look is done with wrapper classes around the handoff orb; `noma-orb.css` stays byte-identical to the design handoff.
- The orb window stays 176 pt for every size: the visual scales inside it, so placement and the panel anchor don't change.

## Changelog

- 2026-11-11 — created: menu-bar count, orb attention glow, orb position per screen, panel search and capture, quiet hours, orb size and style, Open Today / Search / Open last capture shortcuts.
