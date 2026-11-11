# Accessibility

**Status:** Shipped (Unreleased) · **Phase:** 4

## Scope

Baseline every view and floating window keeps:

1. **Keyboard.** Every interactive element is reachable with Tab and shows a visible focus ring (`:focus-visible`, 2 px lavender outline; borderless inline fields get a lavender underline; menu items an inset ring). Custom controls act on Return/Space: segmented choices in Settings follow the radio-group pattern (one tab stop, arrows change the choice), canvas group labels rename on Return or F2, the orb opens the panel (or stops listening) on Space/Return.
2. **Names.** Icon-only buttons carry an `aria-label` (from `copy.ts`, so they are translated); regions and dialogs are labelled.
3. **Dialogs.** Command palette, Quick add, Ask NOMA, the confirm dialog and onboarding are `role="dialog"`/`alertdialog` with `aria-modal="true"`, trap Tab inside (`useFocusTrap`) and give focus back to where it was when they close.
4. **Live feedback.** Toasts, voice-overlay status and capture errors are announced (`role="status"` / `role="alert"` / `aria-live`).
5. **Reduced motion.** With macOS _Reduce motion_ on, CSS animations and transitions collapse to an instant (global rule in `global.css`); canvas zoom buttons jump instead of animating and the graph view lays out off-screen and draws the settled result.
6. **Contrast.** Small text and accents reach 4.5:1 on every surface of both themes. Light theme tokens were darkened from the handoff values: `--noma-text-2` `#6B6575`, `--noma-text-muted` `#6E6878`, `--noma-lavender` `#7255D0`, `--noma-reminder-text` `#96600F`, `--noma-urgent-text` `#BD3F47`; dark `--noma-text-muted` is `#8C8698`.

## Changelog

- 2026-11-11 — created: keyboard, names, modal dialogs, live regions, reduced motion and contrast pass.
