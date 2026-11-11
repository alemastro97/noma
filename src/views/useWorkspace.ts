import { useEffect, useMemo, useState } from "react";
import type { Item, Progress, Project } from "../api/types";
import { setPinned } from "../stores/cards";
import { matchesFilter } from "../lib/filterSpec";
import { useFiltersStore } from "../stores/filters";
import { useItemsStore } from "../stores/items";
import { useProjectsStore } from "../stores/projects";
import { selectCard, useSelectionStore } from "../stores/selection";
import { useUiStore } from "../stores/ui";

/** Current time, refreshed every 30 s so relative labels ("in 2h 10m") stay true. */
export function useNow(intervalMs = 30_000): Date {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const t = setInterval(() => setNow(new Date()), intervalMs);
    return () => clearInterval(t);
  }, [intervalMs]);
  return now;
}

export interface CardHandlers {
  onToggle: (id: string) => void;
  onOpen: (id: string) => void;
  onSelect: (id: string) => void;
  onDuplicate: (id: string) => void;
  onDelete: (id: string) => void;
  onPin: (id: string) => void;
}

/** Stable handlers shared by every view (stable identity keeps memoized cards from re-rendering). */
export function useCardHandlers(): CardHandlers {
  return useMemo(
    () => ({
      onToggle: (id) => void useItemsStore.getState().toggleComplete(id),
      onOpen: (id) => useUiStore.getState().openDetail(id),
      onSelect: (id) => selectCard(id),
      onDuplicate: (id) => void useItemsStore.getState().duplicate(id),
      onDelete: (id) => useItemsStore.getState().remove(id),
      onPin: (id) => void setPinned(id, !useItemsStore.getState().items[id]?.pinned),
    }),
    [],
  );
}

export interface WorkspaceData {
  items: Item[];
  projectsById: Record<string, Project>;
  progress: Record<string, Progress>;
  loading: boolean;
  freshId: string | null;
  selectedId: string | null;
  /** Every selected card (multi-select), including `selectedId`. */
  selection: ReadonlySet<string>;
}

export function useWorkspaceData(): WorkspaceData {
  const itemsById = useItemsStore((s) => s.items);
  const order = useItemsStore((s) => s.order);
  const loading = useItemsStore((s) => s.loading);
  const freshId = useItemsStore((s) => s.freshId);
  const projects = useProjectsStore((s) => s.projects);
  const progress = useProjectsStore((s) => s.progress);
  const selectedId = useUiStore((s) => s.selectedId);
  const selectedIds = useSelectionStore((s) => s.ids);
  const spec = useFiltersStore((s) => s.spec);

  const items = useMemo(
    () => order.map((id) => itemsById[id]).filter((i): i is Item => !!i && matchesFilter(i, spec)),
    [order, itemsById, spec],
  );
  const selection = useMemo(() => {
    const set = new Set(selectedIds);
    if (selectedId) set.add(selectedId);
    return set;
  }, [selectedIds, selectedId]);
  const projectsById = useMemo(
    () => Object.fromEntries(projects.map((p) => [p.id, p])),
    [projects],
  );
  return { items, projectsById, progress, loading, freshId, selectedId, selection };
}

/** Progress for a project card: the referenced project's numbers. */
export function progressFor(item: Item, progress: Record<string, Progress>): Progress | undefined {
  return item.kind === "project_ref" && item.projectId ? progress[item.projectId] : undefined;
}
