# NOMA — design handoff

NOMA is a personal AI desktop companion. Tagline: **YOUR SECOND MIND.** Alternative: *Think freely. NOMA remembers.*

This folder is the visual reference for building the app. Open the files in `screens/` in a browser — each is a self-contained static HTML mockup (only Google Fonts is loaded remotely).

## Contents

| File | What it is |
|---|---|
| `tokens.css` | Colors, radii, shadows, glass, type, motion as CSS variables. Dark default, `[data-theme="light"]` override. |
| `noma-orb.css` | Production-ready CSS for the **NomaOrb** component and its 5 states. |
| `screens/Main.html` | Brand identity: wordmark, N symbol, palette, type scale. |
| `screens/NomaOrb.html` | Orb states (idle, listening, processing, success, error) and size ladder 96 → 20px. |
| `screens/Workspace.html` | Main app, Canvas mode: sidebar, header, free-positioned cards, connections. |
| `screens/Cards.html` | Card system: task, reminder, note, idea, project, event + default/hover/selected/completed. |
| `screens/Desktop.html` | Desktop presence: voice capture overlay, reminder toast, quick capture, mini panel, floating orb. |
| `screens/Light.html` | Light theme, List mode (Today) + empty states. |
| `screens/Assets.html` | App icons (dark, light, orb), small sizes, splash screen. |

## Implementation notes

- **Font:** Geist (UI), Geist Mono (dates, times, counts). Sentence case; uppercase only for the tagline.
- **Icons:** Lucide, stroke 1.75, 15–18px.
- **N symbol (SVG, 48 viewBox):**
  ```svg
  <svg viewBox="0 0 48 48" fill="none">
    <path d="M14 35V15.5a1.5 1.5 0 0 1 2.7-.9L31.3 33.4a1.5 1.5 0 0 0 2.7-.9V20" stroke="currentColor" stroke-width="4.2" stroke-linecap="round" stroke-linejoin="round"/>
    <circle cx="34" cy="12.2" r="3.6" fill="#A78BFA"/>
  </svg>
  ```
  Thicken the stroke at small sizes (4.6 @64px, 5.4 @32px, 6.5 @16px). Dot is `#B9A3FF` on the dark app icon, `#8065DC` on light.
- **Cards:** one shell — radius 14, padding 16/18, 1px border `#302D3C`, shadow `0 10px 30px rgba(0,0,0,.28)`, hover lifts 2px over 200ms. Type is signalled by a colored icon + tinted border, never a full colored fill. Idea cards get a soft rose radial wash and italic title.
- **Glass** only for floating desktop elements (overlay, toasts, quick capture, panel). Never for in-app cards.
- **Canvas background:** 24px dot grid (`#262432` dots) plus one very faint violet radial glow.
- **Motion:** 150–300ms, ease `cubic-bezier(.2,.8,.2,1)`. New cards fade + scale from .97 in 280ms. Connection lines use a slow dashed flow. Respect `prefers-reduced-motion` everywhere.
- **Contrast:** small text uses `#A6A0B5`, not `#777184` (which fails 4.5:1 on the dark bg).
- **Shortcuts shown in mockups:** ⌥ Space voice capture, ⌘⇧N quick capture, ⌘K search, Esc / ↵ / Tab in the overlay.
- **Empty state copy:** "Your mind is clear. For now." · "Nothing demanding your attention." · "Every great thing starts somewhere."

## Not designed yet

Kanban, Calendar and Timeline views (only present in the view switcher), the expanded state of the floating orb panel, settings.
