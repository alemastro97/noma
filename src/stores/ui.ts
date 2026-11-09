import { create } from "zustand";
import { persist } from "zustand/middleware";

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

export interface UiState {
  target: NavTarget;
  viewMode: ViewMode;
  sidebarCollapsed: boolean;
  setTarget: (target: NavTarget) => void;
  setViewMode: (mode: ViewMode) => void;
  toggleSidebar: () => void;
}

export const useUiStore = create<UiState>()(
  persist(
    (set) => ({
      target: { kind: "inbox" },
      viewMode: "canvas",
      sidebarCollapsed: false,
      setTarget: (target) => set({ target }),
      setViewMode: (mode) => {
        if (AVAILABLE_VIEW_MODES.includes(mode)) set({ viewMode: mode });
      },
      toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
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
