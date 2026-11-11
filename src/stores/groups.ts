/** Labelled group frames on the current board's canvas. */
import { create } from "zustand";
import * as api from "../api/workspace";
import type { CanvasGroup, GroupPatch } from "../api/workspace";
import { record } from "./history";
import { toastError } from "./toasts";

type Rect = { x: number; y: number; w: number; h: number };

interface GroupsState {
  boardId: string | null;
  groups: CanvasGroup[];
  load: (boardId: string | null) => Promise<void>;
  create: (rect: Rect, label: string, opts?: { record?: boolean }) => Promise<CanvasGroup | null>;
  update: (id: string, patch: GroupPatch, opts?: { record?: boolean }) => Promise<void>;
  remove: (id: string, opts?: { record?: boolean }) => Promise<void>;
}

let seq = 0;

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export const useGroupsStore = create<GroupsState>()((set, get) => ({
  boardId: null,
  groups: [],

  async load(boardId) {
    const mine = ++seq;
    if (!boardId) {
      set({ boardId: null, groups: [] });
      return;
    }
    if (get().boardId !== boardId) set({ boardId, groups: [] });
    try {
      const groups = await api.listGroups(boardId);
      if (mine === seq) set({ boardId, groups: groups ?? [] });
    } catch (err) {
      if (mine === seq) toastError(message(err));
    }
  },

  async create(rect, label, opts = {}) {
    const boardId = get().boardId;
    if (!boardId) return null;
    try {
      const g = await api.createGroup(boardId, label, rect);
      set((s) => ({ groups: [...s.groups, g] }));
      if (opts.record !== false) {
        // A re-created frame gets a new id; keep following it.
        let id = g.id;
        record({
          action: "group",
          undo: () => get().remove(id, { record: false }),
          redo: async () => {
            const again = await get().create(rect, label, { record: false });
            if (again) id = again.id;
          },
        });
      }
      return g;
    } catch (err) {
      toastError(message(err));
      return null;
    }
  },

  async update(id, patch, opts = {}) {
    const before = get().groups.find((g) => g.id === id);
    if (!before) return;
    if (opts.record !== false) {
      const back: GroupPatch = {};
      for (const k of Object.keys(patch) as (keyof GroupPatch)[]) {
        (back as Record<string, unknown>)[k] = before[k];
      }
      record({
        action: "group",
        undo: () => get().update(id, back, { record: false }),
        redo: () => get().update(id, patch, { record: false }),
      });
    }
    set((s) => ({ groups: s.groups.map((g) => (g.id === id ? { ...g, ...patch } : g)) }));
    try {
      const saved = await api.updateGroup(id, patch);
      set((s) => ({ groups: s.groups.map((g) => (g.id === id ? saved : g)) }));
    } catch (err) {
      set((s) => ({ groups: s.groups.map((g) => (g.id === id ? before : g)) }));
      toastError(message(err));
    }
  },

  async remove(id, opts = {}) {
    const before = get().groups;
    const gone = before.find((g) => g.id === id);
    if (!gone) return;
    set({ groups: before.filter((g) => g.id !== id) });
    if (opts.record !== false) {
      let current = id;
      const rect = { x: gone.x, y: gone.y, w: gone.w, h: gone.h };
      record({
        action: "group",
        undo: async () => {
          const again = await get().create(rect, gone.label, { record: false });
          if (again) current = again.id;
        },
        redo: () => get().remove(current, { record: false }),
      });
    }
    try {
      await api.deleteGroup(id);
    } catch (err) {
      set({ groups: before });
      toastError(message(err));
    }
  },
}));

/** Ids of the nodes whose center lies inside `rect`. */
export function insideRect(
  rect: Rect,
  nodes: { id: string; x: number; y: number; w: number; h: number }[],
): string[] {
  return nodes
    .filter((n) => {
      const cx = n.x + n.w / 2;
      const cy = n.y + n.h / 2;
      return cx >= rect.x && cx <= rect.x + rect.w && cy >= rect.y && cy <= rect.y + rect.h;
    })
    .map((n) => n.id);
}
