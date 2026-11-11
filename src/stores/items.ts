import type { UnlistenFn } from "@tauri-apps/api/event";
import { create } from "zustand";
import { onItemsChanged } from "../api/events";
import { trashItem } from "../api/cards";
import * as api from "../api/items";
import { cardsCopy } from "../copy";
import {
  EMPTY_COUNTS,
  type Counts,
  type Item,
  type ItemFilter,
  type ItemPatch,
  type NewItem,
  type Status,
} from "../api/types";
import { useBoardStore } from "./board";
import { useProjectsStore } from "./projects";
import { TOAST_MS, toastError, useToastsStore } from "./toasts";

export interface LoadOptions {
  includeDone?: boolean;
}

interface PendingDelete {
  item: Item;
  index: number;
  timer: ReturnType<typeof setTimeout>;
}

export interface ItemsState {
  items: Record<string, Item>;
  /** Ids in the order the last `load` returned them (new items are prepended). */
  order: string[];
  counts: Counts;
  loading: boolean;
  error: string | null;
  lastFilter: ItemFilter | null;
  lastOptions: LoadOptions;
  /** Most recently created id, for the arrival animation. */
  freshId: string | null;
  load: (filter: ItemFilter, options?: LoadOptions) => Promise<void>;
  reload: () => Promise<void>;
  create: (input: NewItem) => Promise<Item | null>;
  update: (id: string, patch: ItemPatch) => Promise<void>;
  complete: (id: string) => Promise<void>;
  toggleComplete: (id: string) => Promise<void>;
  setStatus: (id: string, status: Status) => Promise<void>;
  duplicate: (id: string) => Promise<Item | null>;
  /** Removes now, deletes on the server after the undo window. */
  remove: (id: string) => void;
  refreshCounts: () => Promise<void>;
  subscribe: () => Promise<UnlistenFn>;
}

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

// Latest-request-wins guard for `load`.
let loadSeq = 0;
const pendingDeletes = new Map<string, PendingDelete>();

export function isPendingDelete(id: string): boolean {
  return pendingDeletes.has(id);
}

function replaceOne(s: ItemsState, item: Item): Partial<ItemsState> {
  return { items: { ...s.items, [item.id]: item } };
}

export const useItemsStore = create<ItemsState>()((set, get) => ({
  items: {},
  order: [],
  counts: EMPTY_COUNTS,
  loading: false,
  error: null,
  lastFilter: null,
  lastOptions: {},
  freshId: null,

  async load(filter, options = {}) {
    const seq = ++loadSeq;
    set({ loading: true, error: null, lastFilter: filter, lastOptions: options });
    try {
      const list = (await api.listItems(filter, options.includeDone ?? false)).filter(
        (i) => !pendingDeletes.has(i.id),
      );
      if (seq !== loadSeq) return;
      const items: Record<string, Item> = {};
      for (const it of list) items[it.id] = it;
      set({ items, order: list.map((i) => i.id), loading: false });
    } catch (err) {
      if (seq !== loadSeq) return;
      set({ loading: false, error: message(err) });
    }
  },

  async reload() {
    const { lastFilter, lastOptions } = get();
    if (lastFilter) await get().load(lastFilter, lastOptions);
  },

  async create(input) {
    try {
      const created = await api.createItem(input);
      set((s) => ({
        items: { ...s.items, [created.id]: created },
        order: [created.id, ...s.order.filter((id) => id !== created.id)],
        freshId: created.id,
      }));
      return created;
    } catch (err) {
      toastError(message(err));
      return null;
    }
  },

  async update(id, patch) {
    const before = get().items[id];
    if (before) {
      const { tags, ...fields } = patch;
      const optimistic: Item = { ...before, ...fields, tags: tags ?? before.tags };
      set((s) => replaceOne(s, optimistic));
    }
    try {
      const updated = await api.updateItem(id, patch);
      set((s) => replaceOne(s, updated));
    } catch (err) {
      if (before) set((s) => replaceOne(s, before));
      toastError(message(err));
    }
  },

  async complete(id) {
    const before = get().items[id];
    // Recurring items advance on the server instead of finishing: no optimistic "done".
    if (before && !before.recurrence) {
      set((s) =>
        replaceOne(s, { ...before, status: "done", completedAt: new Date().toISOString() }),
      );
    }
    try {
      const updated = await api.completeItem(id);
      set((s) => replaceOne(s, updated));
      if (before?.recurrence && updated.status !== "done") {
        useToastsStore.getState().push({ message: "Done for now — next one scheduled." });
      }
    } catch (err) {
      if (before) set((s) => replaceOne(s, before));
      toastError(message(err));
    }
  },

  async toggleComplete(id) {
    const item = get().items[id];
    if (!item) return;
    if (item.status === "done") await get().update(id, { status: "inbox" });
    else await get().complete(id);
  },

  async setStatus(id, status) {
    if (status === "done") await get().complete(id);
    else await get().update(id, { status });
  },

  async duplicate(id) {
    try {
      const copy = await api.duplicateItem(id);
      set((s) => {
        const index = s.order.indexOf(id);
        const order = [...s.order];
        order.splice(index < 0 ? 0 : index + 1, 0, copy.id);
        return { items: { ...s.items, [copy.id]: copy }, order, freshId: copy.id };
      });
      return copy;
    } catch (err) {
      toastError(message(err));
      return null;
    }
  },

  remove(id) {
    const item = get().items[id];
    if (!item || pendingDeletes.has(id)) return;
    const index = get().order.indexOf(id);
    set((s) => {
      const items = { ...s.items };
      delete items[id];
      return { items, order: s.order.filter((x) => x !== id) };
    });

    const commit = async () => {
      pendingDeletes.delete(id);
      try {
        await trashItem(id);
      } catch (err) {
        toastError(message(err));
        void get().reload();
      }
    };
    const timer = setTimeout(() => void commit(), TOAST_MS);
    pendingDeletes.set(id, { item, index, timer });

    useToastsStore.getState().push({
      message: cardsCopy.trashed(item.title),
      action: {
        label: "Undo",
        run: () => {
          const pending = pendingDeletes.get(id);
          if (!pending) return;
          clearTimeout(pending.timer);
          pendingDeletes.delete(id);
          set((s) => {
            const order = s.order.filter((x) => x !== id);
            order.splice(Math.min(pending.index, order.length), 0, id);
            return { items: { ...s.items, [id]: pending.item }, order };
          });
        },
      },
    });
  },

  async refreshCounts() {
    try {
      const counts = await api.getCounts();
      if (counts) set({ counts });
    } catch (err) {
      set({ error: message(err) });
    }
  },

  subscribe() {
    // Coalesce bursts (a drag, a seed) into one refresh per frame.
    let queued = false;
    return onItemsChanged(() => {
      if (queued) return;
      queued = true;
      setTimeout(() => {
        queued = false;
        void get().reload();
        void get().refreshCounts();
        void useProjectsStore.getState().load();
        void useBoardStore.getState().reload();
      }, 16);
    });
  },
}));

/** Flushes pending deletes immediately (used on window unload). */
export function flushPendingDeletes(): void {
  for (const [id, p] of pendingDeletes) {
    clearTimeout(p.timer);
    pendingDeletes.delete(id);
    void trashItem(id);
  }
}
