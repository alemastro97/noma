import type { UnlistenFn } from "@tauri-apps/api/event";
import { create } from "zustand";
import { onItemsChanged } from "../api/events";
import { restoreItem, trashItem } from "../api/cards";
import * as api from "../api/items";
import { cardsCopy, uiCopy } from "../copy";
import {
  EMPTY_COUNTS,
  type Counts,
  type Item,
  type ItemFilter,
  type ItemPatch,
  type NewItem,
  type Status,
} from "../api/types";
import { inversePatch } from "../lib/inversePatch";
import { useBoardStore } from "./board";
import { lastEntry, record, undoEntry } from "./history";
import { useProjectsStore } from "./projects";
import { toastError, useToastsStore } from "./toasts";

export interface LoadOptions {
  includeDone?: boolean;
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
  /** Moves the card to the Trash at once; Undo (toast or ⌘Z) restores it. `quiet` skips the toast. */
  remove: (id: string, opts?: { quiet?: boolean }) => void;
  refreshCounts: () => Promise<void>;
  subscribe: () => Promise<UnlistenFn>;
}

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

// Latest-request-wins guard for `load`.
let loadSeq = 0;
/** Cards with a write in flight: a reload must not overwrite their optimistic state. */
const inflight = new Map<string, number>();

function begin(id: string): void {
  inflight.set(id, (inflight.get(id) ?? 0) + 1);
}

function end(id: string): void {
  const n = (inflight.get(id) ?? 1) - 1;
  if (n <= 0) inflight.delete(id);
  else inflight.set(id, n);
}

function same(a: Item | undefined, b: Item): a is Item {
  return !!a && a.updatedAt === b.updatedAt && JSON.stringify(a) === JSON.stringify(b);
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
    await fetchInto(filter, options, false);
  },

  async reload() {
    const { lastFilter, lastOptions } = get();
    if (lastFilter) await fetchInto(lastFilter, lastOptions, true);
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
      const inverse = inversePatch(before, patch);
      record({
        action: "edit",
        undo: () => get().update(id, inverse),
        redo: () => get().update(id, patch),
      });
      const { tags, ...fields } = patch;
      const optimistic: Item = { ...before, ...fields, tags: tags ?? before.tags };
      set((s) => replaceOne(s, optimistic));
    }
    begin(id);
    try {
      const updated = await api.updateItem(id, patch);
      set((s) => replaceOne(s, updated));
    } catch (err) {
      if (before) set((s) => replaceOne(s, before));
      toastError(message(err));
    } finally {
      end(id);
    }
  },

  async complete(id) {
    const before = get().items[id];
    if (before) {
      const back: ItemPatch = { status: before.status };
      if (before.recurrence)
        Object.assign(back, { dueAt: before.dueAt, remindAt: before.remindAt });
      record({
        action: "complete",
        undo: () => get().update(id, back),
        redo: () => get().complete(id),
      });
    }
    // Recurring items advance on the server instead of finishing: no optimistic "done".
    if (before && !before.recurrence) {
      set((s) =>
        replaceOne(s, { ...before, status: "done", completedAt: new Date().toISOString() }),
      );
    }
    begin(id);
    try {
      const updated = await api.completeItem(id);
      set((s) => replaceOne(s, updated));
      if (before?.recurrence && updated.status !== "done") {
        useToastsStore.getState().push({ message: uiCopy.recurringDone });
      }
    } catch (err) {
      if (before) set((s) => replaceOne(s, before));
      toastError(message(err));
    } finally {
      end(id);
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

  remove(id, opts = {}) {
    const item = get().items[id];
    if (!item) return;
    record({
      action: "delete",
      undo: () => restoreDeleted(id),
      redo: () => get().remove(id, { quiet: true }),
    });
    const entry = lastEntry();
    set((s) => {
      const items = { ...s.items };
      delete items[id];
      return { items, order: s.order.filter((x) => x !== id) };
    });
    // The Trash keeps it recoverable, so there is nothing to wait for.
    void trashItem(id).catch((err) => {
      toastError(message(err));
      void get().reload();
    });

    if (opts.quiet) return;
    useToastsStore.getState().push({
      message: cardsCopy.trashed(item.title),
      action: { label: uiCopy.undo, run: () => void undoEntry(entry) },
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
    // Trailing debounce: a burst (bulk action, import, AI capture) is one refresh.
    let timer: ReturnType<typeof setTimeout> | undefined;
    return onItemsChanged(() => {
      clearTimeout(timer);
      timer = setTimeout(() => {
        void get().reload();
        void get().refreshCounts();
        void useProjectsStore.getState().load();
        void useBoardStore.getState().reload();
      }, 40);
    });
  },
}));

/**
 * Fetches a filter into the store. Unchanged cards keep their object identity
 * (so memoized cards do not re-render) and cards with a write in flight keep
 * their optimistic state. A background `quiet` reload never flips `loading`.
 */
async function fetchInto(filter: ItemFilter, options: LoadOptions, quiet: boolean): Promise<void> {
  const { setState: set, getState: get } = useItemsStore;
  const seq = ++loadSeq;
  if (quiet) set({ lastFilter: filter, lastOptions: options });
  else set({ loading: true, error: null, lastFilter: filter, lastOptions: options });
  try {
    const list = await api.listItems(filter, options.includeDone ?? false);
    if (seq !== loadSeq) return;
    const prev = get();
    const items: Record<string, Item> = {};
    let changed = list.length !== prev.order.length;
    list.forEach((it, i) => {
      const old = prev.items[it.id];
      const keep = old && (inflight.has(it.id) || same(old, it)) ? old : it;
      items[it.id] = keep;
      if (keep !== old || prev.order[i] !== it.id) changed = true;
    });
    if (!changed) {
      if (prev.loading) set({ loading: false });
      return;
    }
    const order = list.map((i) => i.id);
    const sameOrder =
      order.length === prev.order.length && order.every((id, i) => id === prev.order[i]);
    set({ items, order: sameOrder ? prev.order : order, loading: false });
  } catch (err) {
    if (seq !== loadSeq) return;
    set({ loading: false, error: message(err) });
  }
}

/** Undo of a delete: bring the card back from the Trash. */
async function restoreDeleted(id: string): Promise<void> {
  try {
    await restoreItem(id);
    await useItemsStore.getState().reload();
  } catch (err) {
    toastError(message(err));
  }
}

// The arrival highlight is a one-shot: clear it so it does not replay on remount.
useItemsStore.subscribe((s, prev) => {
  const id = s.freshId;
  if (!id || id === prev.freshId) return;
  setTimeout(() => {
    if (useItemsStore.getState().freshId === id) useItemsStore.setState({ freshId: null });
  }, 1600);
});
