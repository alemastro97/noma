import { invoke } from "@tauri-apps/api/core";
import type { Iso, ItemType, Link, Place, Status } from "./types";

/** A refinement over the cards a view shows. Every set field must match. */
export interface FilterSpec {
  /** Substring of title or notes (case-insensitive). */
  text?: string;
  types?: ItemType[];
  tags?: string[];
  /** Project ids; "none" matches cards without a project. */
  projectIds?: string[];
  statuses?: Status[];
  /** Local calendar days, YYYY-MM-DD, inclusive, on the card's date (due or reminder). */
  from?: string;
  to?: string;
}

export interface SavedFilter {
  id: string;
  name: string;
  spec: FilterSpec;
  createdAt: Iso;
}

export interface CanvasGroup {
  id: string;
  boardId: string;
  label: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

export type GroupPatch = Partial<Pick<CanvasGroup, "label" | "x" | "y" | "w" | "h">>;

/** Where a card was before a move (undo puts it back there). */
export interface PriorPlace {
  id: string;
  place: Place;
  boardId: string | null;
  x: number | null;
  y: number | null;
}

/** Opaque rows of a card, taken before deleting it so it can be restored. */
export type ItemSnapshot = Record<string, unknown>;

export function listFilters(): Promise<SavedFilter[]> {
  return invoke<SavedFilter[]>("filters_list");
}

export function createFilter(name: string, spec: FilterSpec): Promise<SavedFilter> {
  return invoke<SavedFilter>("filters_create", { name, spec });
}

export function deleteFilter(id: string): Promise<void> {
  return invoke<void>("filters_delete", { id });
}

export function listGroups(boardId: string): Promise<CanvasGroup[]> {
  return invoke<CanvasGroup[]>("groups_list", { boardId });
}

export function createGroup(
  boardId: string,
  label: string,
  rect: { x: number; y: number; w: number; h: number },
): Promise<CanvasGroup> {
  return invoke<CanvasGroup>("groups_create", { boardId, label, ...rect });
}

export function updateGroup(id: string, patch: GroupPatch): Promise<CanvasGroup> {
  return invoke<CanvasGroup>("groups_update", { id, patch });
}

export function deleteGroup(id: string): Promise<void> {
  return invoke<void>("groups_delete", { id });
}

export function listAllLinks(): Promise<Link[]> {
  return invoke<Link[]>("links_all");
}

export function moveItems(ids: string[], place: Place): Promise<PriorPlace[]> {
  return invoke<PriorPlace[]>("items_move", { ids, place });
}

export function unmoveItems(prior: PriorPlace[]): Promise<void> {
  return invoke<void>("items_unmove", { prior });
}

export function snapshotItems(ids: string[]): Promise<ItemSnapshot[]> {
  return invoke<ItemSnapshot[]>("items_snapshot", { ids });
}

export function restoreItems(snapshots: ItemSnapshot[]): Promise<string[]> {
  return invoke<string[]>("items_restore", { snapshots });
}
