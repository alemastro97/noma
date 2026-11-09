import { create } from "zustand";
import * as api from "../api/boards";
import type { Link } from "../api/types";
import { toastError } from "./toasts";

export interface Point {
  x: number;
  y: number;
}

interface BoardState {
  boardId: string | null;
  positions: Record<string, Point>;
  links: Link[];
  load: (boardId: string | null) => Promise<void>;
  reload: () => Promise<void>;
  move: (itemId: string, point: Point) => Promise<void>;
  connect: (from: string, to: string) => Promise<void>;
  disconnect: (linkId: string) => Promise<void>;
}

let seq = 0;

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export const useBoardStore = create<BoardState>()((set, get) => ({
  boardId: null,
  positions: {},
  links: [],

  async load(boardId) {
    const mine = ++seq;
    if (!boardId) {
      set({ boardId: null, positions: {}, links: [] });
      return;
    }
    if (get().boardId !== boardId) set({ boardId, positions: {}, links: [] });
    try {
      const [placements, links] = await Promise.all([
        api.listPlacements(boardId),
        api.listLinks(boardId),
      ]);
      if (mine !== seq) return;
      const positions: Record<string, Point> = {};
      for (const p of placements ?? []) positions[p.itemId] = { x: p.x, y: p.y };
      set({ boardId, positions, links: links ?? [] });
    } catch (err) {
      if (mine === seq) toastError(message(err));
    }
  },

  async reload() {
    await get().load(get().boardId);
  },

  async move(itemId, point) {
    const boardId = get().boardId;
    if (!boardId) return;
    set((s) => ({ positions: { ...s.positions, [itemId]: point } }));
    try {
      await api.setPlacement(itemId, boardId, point.x, point.y);
    } catch (err) {
      toastError(message(err));
    }
  },

  async connect(from, to) {
    if (from === to) return;
    try {
      const link = await api.createLink(from, to);
      set((s) => (s.links.some((l) => l.id === link.id) ? s : { links: [...s.links, link] }));
    } catch (err) {
      toastError(message(err));
    }
  },

  async disconnect(linkId) {
    const before = get().links;
    set({ links: before.filter((l) => l.id !== linkId) });
    try {
      await api.deleteLink(linkId);
    } catch (err) {
      set({ links: before });
      toastError(message(err));
    }
  },
}));
