/** Actions on many cards at once (bulk bar, drag to sidebar), each one undo step. */
import type { Iso, Place } from "../api/types";
import { moveItems, unmoveItems, type PriorPlace } from "../api/workspace";
import { workspaceCopy } from "../copy";
import { batchHistory, record } from "./history";
import { cancelPendingDelete, useItemsStore } from "./items";
import { useSelectionStore } from "./selection";
import { toastError, useToastsStore } from "./toasts";
import { useUiStore } from "./ui";

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export function bulkComplete(ids: string[]): void {
  const items = useItemsStore.getState();
  batchHistory("bulk", () => {
    for (const id of ids) {
      if (items.items[id] && items.items[id].status !== "done") void items.complete(id);
    }
  });
}

export function bulkSetDate(ids: string[], dueAt: Iso | null): void {
  const items = useItemsStore.getState();
  batchHistory("bulk", () => {
    for (const id of ids) if (items.items[id]) void items.update(id, { dueAt });
  });
}

export function bulkDelete(ids: string[]): void {
  const items = useItemsStore.getState();
  const present = ids.filter((id) => items.items[id]);
  if (present.length === 0) return;
  if (present.length === 1) {
    items.remove(present[0]!);
  } else {
    batchHistory("delete", () => {
      for (const id of present) items.remove(id, { quiet: true });
    });
    useToastsStore.getState().push({
      message: workspaceCopy.bulkDeleted(present.length),
      action: {
        label: workspaceCopy.undo,
        run: () => present.forEach((id) => cancelPendingDelete(id)),
      },
    });
  }
  useSelectionStore.getState().clear();
  useUiStore.getState().select(null);
}

/** Moves cards to a project, personal board or the Inbox. */
export async function moveCards(ids: string[], place: Place, placeName: string): Promise<void> {
  if (ids.length === 0) return;
  try {
    let prior: PriorPlace[] = await moveItems(ids, place);
    record({
      action: "move",
      undo: () => unmoveItems(prior),
      redo: async () => {
        prior = await moveItems(ids, place);
      },
    });
    useToastsStore.getState().push({ message: workspaceCopy.bulkMoved(ids.length, placeName) });
    useSelectionStore.getState().clear();
  } catch (err) {
    toastError(message(err));
  }
}
