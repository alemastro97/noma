import type { UnlistenFn } from "@tauri-apps/api/event";
import { create } from "zustand";
import { onItemsChanged } from "../api/events";
import * as api from "../api/items";
import {
  EMPTY_COUNTS,
  type Counts,
  type Item,
  type ItemFilter,
  type ItemPatch,
  type NewItem,
} from "../api/types";

export interface ItemsState {
  items: Record<string, Item>;
  /** Ids in the order the last `load` returned them (new items are prepended). */
  order: string[];
  counts: Counts;
  loading: boolean;
  error: string | null;
  lastFilter: ItemFilter | null;
  load: (filter: ItemFilter) => Promise<void>;
  reload: () => Promise<void>;
  create: (input: NewItem) => Promise<Item>;
  update: (id: string, patch: ItemPatch) => Promise<void>;
  complete: (id: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
  refreshCounts: () => Promise<void>;
  /** Subscribes to backend change events; resolves to the unlisten function. */
  subscribe: () => Promise<UnlistenFn>;
}

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export const useItemsStore = create<ItemsState>()((set, get) => ({
  items: {},
  order: [],
  counts: EMPTY_COUNTS,
  loading: false,
  error: null,
  lastFilter: null,

  async load(filter) {
    set({ loading: true, error: null, lastFilter: filter });
    try {
      const list = await api.listItems(filter);
      const items: Record<string, Item> = {};
      for (const it of list) items[it.id] = it;
      set({ items, order: list.map((i) => i.id), loading: false });
    } catch (err) {
      set({ loading: false, error: message(err) });
    }
  },

  async reload() {
    const filter = get().lastFilter;
    if (filter) await get().load(filter);
  },

  async create(input) {
    const created = await api.createItem(input);
    set((s) => ({
      items: { ...s.items, [created.id]: created },
      order: [created.id, ...s.order.filter((id) => id !== created.id)],
    }));
    return created;
  },

  async update(id, patch) {
    const before = get().items[id];
    if (before) {
      const optimistic: Item = { ...before };
      if (patch.title !== undefined) optimistic.title = patch.title;
      if (patch.status !== undefined) optimistic.status = patch.status;
      if (patch.priority !== undefined) optimistic.priority = patch.priority;
      set((s) => ({ items: { ...s.items, [id]: optimistic } }));
    }
    try {
      const updated = await api.updateItem(id, patch);
      set((s) => ({ items: { ...s.items, [id]: updated } }));
    } catch (err) {
      if (before) set((s) => ({ items: { ...s.items, [id]: before } }));
      set({ error: message(err) });
    }
  },

  async complete(id) {
    const before = get().items[id];
    if (before) {
      set((s) => ({
        items: {
          ...s.items,
          [id]: { ...before, status: "done", completedAt: new Date().toISOString() },
        },
      }));
    }
    try {
      const updated = await api.completeItem(id);
      set((s) => ({ items: { ...s.items, [id]: updated } }));
    } catch (err) {
      if (before) set((s) => ({ items: { ...s.items, [id]: before } }));
      set({ error: message(err) });
    }
  },

  async remove(id) {
    const before = get();
    set((s) => {
      const items = { ...s.items };
      delete items[id];
      return { items, order: s.order.filter((x) => x !== id) };
    });
    try {
      await api.deleteItem(id);
    } catch (err) {
      set({ items: before.items, order: before.order, error: message(err) });
    }
  },

  async refreshCounts() {
    try {
      const counts = await api.getCounts();
      set({ counts });
    } catch (err) {
      set({ error: message(err) });
    }
  },

  subscribe() {
    return onItemsChanged(() => {
      void get().reload();
      void get().refreshCounts();
    });
  },
}));
