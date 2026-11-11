/**
 * The filter bar's conditions (applied on top of whatever the current view
 * loaded) and the filters saved as sidebar entries. A saved filter is a
 * navigation target that loads every card and applies its conditions.
 */
import { create } from "zustand";
import * as api from "../api/workspace";
import type { FilterSpec, SavedFilter } from "../api/workspace";
import { workspaceCopy } from "../copy";
import { toastError } from "./toasts";
import { useUiStore } from "./ui";

interface FiltersState {
  open: boolean;
  spec: FilterSpec;
  saved: SavedFilter[];
  setOpen: (open: boolean) => void;
  setSpec: (patch: Partial<FilterSpec>) => void;
  reset: () => void;
  load: () => Promise<void>;
  save: (name: string) => Promise<SavedFilter | null>;
  remove: (id: string) => Promise<void>;
}

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/** Drops empty fields so a saved spec stays small and comparable. */
export function compactSpec(spec: FilterSpec): FilterSpec {
  const out: FilterSpec = {};
  if (spec.text?.trim()) out.text = spec.text.trim();
  if (spec.types?.length) out.types = spec.types;
  if (spec.tags?.length) out.tags = spec.tags;
  if (spec.projectIds?.length) out.projectIds = spec.projectIds;
  if (spec.statuses?.length) out.statuses = spec.statuses;
  if (spec.from) out.from = spec.from;
  if (spec.to) out.to = spec.to;
  return out;
}

export const useFiltersStore = create<FiltersState>()((set, get) => ({
  open: false,
  spec: {},
  saved: [],
  setOpen: (open) => set({ open }),
  setSpec: (patch) => set((s) => ({ spec: { ...s.spec, ...patch } })),
  reset: () => set({ spec: {} }),

  async load() {
    try {
      const saved = await api.listFilters();
      set({ saved: saved ?? [] });
      applyTarget();
    } catch (err) {
      toastError(message(err));
    }
  },

  async save(name) {
    try {
      const f = await api.createFilter(name, compactSpec(get().spec));
      set((s) => ({ saved: [...s.saved, f] }));
      return f;
    } catch (err) {
      toastError(message(err));
      return null;
    }
  },

  async remove(id) {
    const before = get().saved;
    set({ saved: before.filter((f) => f.id !== id) });
    try {
      await api.deleteFilter(id);
      const ui = useUiStore.getState();
      if (ui.target.kind === "filter" && ui.target.id === id) ui.setTarget({ kind: "inbox" });
    } catch (err) {
      set({ saved: before });
      toastError(message(err));
    }
  },
}));

export function savedFilterName(id: string): string {
  return useFiltersStore.getState().saved.find((f) => f.id === id)?.name ?? workspaceCopy.filters;
}

/** A saved filter target shows its conditions; any other target starts unfiltered. */
function applyTarget(): void {
  const target = useUiStore.getState().target;
  if (target.kind === "filter") {
    const f = useFiltersStore.getState().saved.find((x) => x.id === target.id);
    useFiltersStore.setState({ spec: f ? { ...f.spec } : {}, open: true });
  } else {
    useFiltersStore.setState({ spec: {} });
  }
}

useUiStore.subscribe((s, prev) => {
  if (s.target !== prev.target) applyTarget();
});
