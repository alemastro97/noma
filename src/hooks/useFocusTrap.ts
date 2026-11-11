import { useEffect, type RefObject } from "react";

const FOCUSABLE = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled]):not([type=hidden])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[contenteditable=true]",
  "[tabindex]:not([tabindex='-1'])",
].join(",");

function focusables(root: HTMLElement): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
    (el) => !el.closest("[inert]") && el.getClientRects().length > 0,
  );
}

/**
 * Keeps Tab / Shift+Tab inside a modal dialog while it is open, moves focus
 * into it if nothing inside has focus yet, and gives focus back to whatever had
 * it before the dialog opened when it closes.
 */
export function useFocusTrap(ref: RefObject<HTMLElement | null>, active = true): void {
  useEffect(() => {
    const root = ref.current;
    if (!active || !root) return;
    const before = document.activeElement as HTMLElement | null;

    // Let the dialog's own autofocus run first; fall back to its first control.
    const raf = requestAnimationFrame(() => {
      if (!root.contains(document.activeElement)) (focusables(root)[0] ?? root).focus();
    });

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Tab") return;
      const items = focusables(root);
      if (items.length === 0) {
        e.preventDefault();
        root.focus();
        return;
      }
      const first = items[0]!;
      const last = items[items.length - 1]!;
      const at = document.activeElement;
      if (e.shiftKey && (at === first || !root.contains(at))) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && (at === last || !root.contains(at))) {
        e.preventDefault();
        first.focus();
      }
    };
    document.addEventListener("keydown", onKeyDown, true);
    return () => {
      cancelAnimationFrame(raf);
      document.removeEventListener("keydown", onKeyDown, true);
      if (before && document.contains(before)) before.focus({ preventScroll: true });
    };
  }, [ref, active]);
}
