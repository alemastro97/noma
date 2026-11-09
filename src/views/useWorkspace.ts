import { useEffect, useMemo, useState } from "react";
import type { Item, Progress, Project } from "../api/types";
import { useItemsStore } from "../stores/items";
import { useProjectsStore } from "../stores/projects";
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
}

/** Stable handlers shared by every view (stable identity keeps memoized cards from re-rendering). */
export function useCardHandlers(): CardHandlers {
  return useMemo(
    () => ({
      onToggle: (id) => void useItemsStore.getState().toggleComplete(id),
      onOpen: (id) => useUiStore.getState().openDetail(id),
      onSelect: (id) => useUiStore.getState().select(id),
      onDuplicate: (id) => void useItemsStore.getState().duplicate(id),
      onDelete: (id) => useItemsStore.getState().remove(id),
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
}

export function useWorkspaceData(): WorkspaceData {
  const itemsById = useItemsStore((s) => s.items);
  const order = useItemsStore((s) => s.order);
  const loading = useItemsStore((s) => s.loading);
  const freshId = useItemsStore((s) => s.freshId);
  const projects = useProjectsStore((s) => s.projects);
  const progress = useProjectsStore((s) => s.progress);
  const selectedId = useUiStore((s) => s.selectedId);

  const items = useMemo(
    () => order.map((id) => itemsById[id]).filter((i): i is Item => !!i),
    [order, itemsById],
  );
  const projectsById = useMemo(
    () => Object.fromEntries(projects.map((p) => [p.id, p])),
    [projects],
  );
  return { items, projectsById, progress, loading, freshId, selectedId };
}

/** Progress for a project card: the referenced project's numbers. */
export function progressFor(item: Item, progress: Record<string, Progress>): Progress | undefined {
  return item.kind === "project_ref" && item.projectId ? progress[item.projectId] : undefined;
}
