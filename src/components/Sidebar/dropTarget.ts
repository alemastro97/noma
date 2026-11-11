/**
 * Dragging cards onto a project, board or the Inbox in the sidebar. List and
 * Kanban use HTML drag and drop; the canvas (pointer drags) asks
 * `placeAtPoint` where its drag ended.
 */
import type { DragEvent } from "react";
import type { Place } from "../../api/types";
import { useSelectionStore } from "../../stores/selection";

/** Same type the Kanban columns already accept (one id). */
export const CARD_DRAG_TYPE = "application/x-noma-item";
/** Every card being dragged (the selection when the dragged card is part of it). */
export const CARDS_DRAG_TYPE = "application/x-noma-items";

export interface DropPlace {
  place: Place;
  name: string;
}

/** Ids a drag of `id` carries: the whole selection when `id` is in it. */
export function dragIdsFor(id: string): string[] {
  const ids = useSelectionStore.getState().ids;
  return ids.length > 1 && ids.includes(id) ? ids : [id];
}

export function startCardDrag(e: DragEvent, id: string): void {
  e.dataTransfer.setData(CARD_DRAG_TYPE, id);
  e.dataTransfer.setData(CARDS_DRAG_TYPE, JSON.stringify(dragIdsFor(id)));
  e.dataTransfer.effectAllowed = "move";
}

export function isCardDrag(e: DragEvent): boolean {
  return e.dataTransfer.types.includes(CARD_DRAG_TYPE);
}

export function draggedIds(e: DragEvent): string[] {
  try {
    const list = JSON.parse(e.dataTransfer.getData(CARDS_DRAG_TYPE) || "[]") as unknown;
    if (Array.isArray(list) && list.length > 0) return list.map(String);
  } catch {
    // fall through to the single id
  }
  const one = e.dataTransfer.getData(CARD_DRAG_TYPE);
  return one ? [one] : [];
}

/** Attributes that mark a sidebar entry as a drop target. */
export function dropAttrs(drop: DropPlace): Record<string, string> {
  return { "data-drop-place": JSON.stringify(drop.place), "data-drop-name": drop.name };
}

function readDrop(el: Element | null): DropPlace | null {
  const host = el?.closest<HTMLElement>("[data-drop-place]");
  if (!host) return null;
  try {
    return {
      place: JSON.parse(host.dataset.dropPlace ?? "") as Place,
      name: host.dataset.dropName ?? "",
    };
  } catch {
    return null;
  }
}

/** The sidebar drop target under a screen point, if any. */
export function placeAtPoint(x: number, y: number): DropPlace | null {
  if (typeof document === "undefined" || !document.elementFromPoint) return null;
  return readDrop(document.elementFromPoint(x, y));
}

/** Highlights the target under a canvas drag (HTML drags highlight themselves). */
export function hoverAtPoint(x: number, y: number): void {
  const host =
    typeof document !== "undefined" && document.elementFromPoint
      ? document.elementFromPoint(x, y)?.closest<HTMLElement>("[data-drop-place]")
      : null;
  document.querySelectorAll<HTMLElement>("[data-drop-over]").forEach((el) => {
    if (el !== host) delete el.dataset.dropOver;
  });
  if (host) host.dataset.dropOver = "true";
}

export function clearHover(): void {
  document.querySelectorAll<HTMLElement>("[data-drop-over]").forEach((el) => {
    delete el.dataset.dropOver;
  });
}
