import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { ItemType, Status } from "../api/types";

export type NavTarget =
  | { kind: "inbox" }
  | { kind: "today" }
  | { kind: "upcoming" }
  | { kind: "ideas" }
  | { kind: "notes" }
  | { kind: "settings" }
  | { kind: "project"; id: string }
  | { kind: "board"; id: string };

export type ViewMode = "canvas" | "kanban" | "list" | "calendar" | "timeline";

export const ALL_VIEW_MODES: ViewMode[] = ["canvas", "kanban", "list", "calendar", "timeline"];
/** Calendar and Timeline are shown disabled ("Soon") until they are designed. */
export const AVAILABLE_VIEW_MODES: ViewMode[] = ["canvas", "kanban", "list"];

export const UI_STORAGE_KEY = "noma.ui";

export interface QuickAddState {
  open: boolean;
  kind?: ItemType;
  status?: Status;
  /** Canvas position (flow coordinates) when opened by double-clicking the board. */
  position?: { x: number; y: number };
}

export type Creating = "project" | "board" | null;

export interface UiState {
  target: NavTarget;
  viewMode: ViewMode;
  sidebarCollapsed: boolean;
  selectedId: string | null;
  detailId: string | null;
  quickAdd: QuickAddState;
  paletteOpen: boolean;
  creating: Creating;
  setTarget: (target: NavTarget) => void;
  setViewMode: (mode: ViewMode) => void;
  toggleSidebar: () => void;
  select: (id: string | null) => void;
  openDetail: (id: string) => void;
  closeDetail: () => void;
  openQuickAdd: (opts?: Omit<QuickAddState, "open">) => void;
  closeQuickAdd: () => void;
  setPaletteOpen: (open: boolean) => void;
  setCreating: (creating: Creating) => void;
}

export const useUiStore = create<UiState>()(
  persist(
    (set) => ({
      target: { kind: "inbox" },
      viewMode: "canvas",
      sidebarCollapsed: false,
      selectedId: null,
      detailId: null,
      quickAdd: { open: false },
      paletteOpen: false,
      creating: null,
      setTarget: (target) => set({ target, selectedId: null }),
      setViewMode: (mode) => {
        if (AVAILABLE_VIEW_MODES.includes(mode)) set({ viewMode: mode });
      },
      toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
      select: (selectedId) => set({ selectedId }),
      openDetail: (id) => set({ detailId: id, selectedId: id }),
      closeDetail: () => set({ detailId: null }),
      openQuickAdd: (opts = {}) => set({ quickAdd: { ...opts, open: true }, paletteOpen: false }),
      closeQuickAdd: () => set({ quickAdd: { open: false } }),
      setPaletteOpen: (paletteOpen) => set({ paletteOpen }),
      setCreating: (creating) =>
        set((s) => ({ creating, sidebarCollapsed: creating ? false : s.sidebarCollapsed })),
    }),
    {
      name: UI_STORAGE_KEY,
      partialize: (s) => ({
        target: s.target,
        viewMode: s.viewMode,
        sidebarCollapsed: s.sidebarCollapsed,
      }),
    },
  ),
);

export function targetKey(target: NavTarget): string {
  return "id" in target ? `${target.kind}:${target.id}` : target.kind;
}

export function sameTarget(a: NavTarget, b: NavTarget): boolean {
  return targetKey(a) === targetKey(b);
}

/** True while the user is typing in a field, so single-key shortcuts stay out of the way. */
export function isTypingTarget(el: EventTarget | null): boolean {
  if (!(el instanceof HTMLElement)) return false;
  const tag = el.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || el.isContentEditable;
}
