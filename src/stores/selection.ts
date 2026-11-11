/**
 * Multi-select. `ui.selectedId` stays the primary (focused) card; this store
 * holds the whole set. ⌘-click toggles a card, ⇧-click selects the range
 * between the anchor and the clicked card in on-screen order.
 */
import { create } from "zustand";
import { useUiStore } from "./ui";

interface SelectionState {
  ids: string[];
  anchor: string | null;
  set: (ids: string[], anchor?: string | null) => void;
  clear: () => void;
}

export const useSelectionStore = create<SelectionState>()((set) => ({
  ids: [],
  anchor: null,
  set: (ids, anchor) => set((s) => ({ ids, anchor: anchor === undefined ? s.anchor : anchor })),
  clear: () => set({ ids: [], anchor: null }),
}));

/** Modifier keys of the latest pointer press (cards only report their id on select). */
const mods = { toggle: false, range: false };
let installed = false;

export function installPointerModifiers(): void {
  if (installed || typeof window === "undefined") return;
  installed = true;
  window.addEventListener(
    "pointerdown",
    (e) => {
      mods.toggle = e.metaKey || e.ctrlKey;
      mods.range = e.shiftKey;
    },
    true,
  );
  // Keep the set in step with the primary selection set elsewhere (Esc, pane
  // click, navigation, opening a card).
  useUiStore.subscribe((s, prev) => {
    if (s.selectedId === prev.selectedId) return;
    const sel = useSelectionStore.getState();
    if (s.selectedId === null) sel.clear();
    else if (!sel.ids.includes(s.selectedId)) sel.set([s.selectedId], s.selectedId);
  });
}

/** Card ids in the order they are on screen (rows, columns, nodes). */
export function visibleOrder(root: ParentNode = document): string[] {
  return Array.from(root.querySelectorAll<HTMLElement>("[data-item-id]"))
    .map((el) => el.dataset.itemId ?? "")
    .filter(Boolean);
}

/** The ids between `a` and `b` (inclusive) in `order`; just `[b]` when either is missing. */
export function rangeBetween(order: string[], a: string, b: string): string[] {
  const i = order.indexOf(a);
  const j = order.indexOf(b);
  if (i < 0 || j < 0) return [b];
  return order.slice(Math.min(i, j), Math.max(i, j) + 1);
}

/** Selects `id`, honouring ⌘ / ⇧ from the click that triggered it (or explicit flags). */
export function selectCard(id: string, flags?: { toggle?: boolean; range?: boolean }): void {
  const toggle = flags?.toggle ?? mods.toggle;
  const range = flags?.range ?? mods.range;
  mods.toggle = false;
  mods.range = false;
  const sel = useSelectionStore.getState();
  const ui = useUiStore.getState();
  const current = sel.ids.length > 0 ? sel.ids : ui.selectedId ? [ui.selectedId] : [];

  if (range && sel.anchor) {
    const span = rangeBetween(visibleOrder(), sel.anchor, id);
    const ids = Array.from(new Set([...current, ...span]));
    sel.set(ids);
    ui.select(id);
    return;
  }
  if (toggle) {
    if (current.includes(id)) {
      const ids = current.filter((x) => x !== id);
      sel.set(ids, ids[ids.length - 1] ?? null);
      ui.select(ids[ids.length - 1] ?? null);
    } else {
      sel.set([...current, id], id);
      ui.select(id);
    }
    return;
  }
  sel.set([id], id);
  ui.select(id);
}

/** The cards an action should apply to: the selection, or just the primary card. */
export function selectedIds(): string[] {
  const ids = useSelectionStore.getState().ids;
  if (ids.length > 0) return ids;
  const one = useUiStore.getState().selectedId;
  return one ? [one] : [];
}
