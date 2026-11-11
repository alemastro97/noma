import { create } from "zustand";
import * as cardsApi from "../api/cards";
import type { Item } from "../api/types";
import { cardsCopy } from "../copy";
import { useItemsStore } from "./items";
import { toastError, useToastsStore } from "./toasts";

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function patchLocal(id: string, fields: Partial<Item>): Item | undefined {
  const before = useItemsStore.getState().items[id];
  if (before) {
    useItemsStore.setState((s) => ({ items: { ...s.items, [id]: { ...before, ...fields } } }));
  }
  return before;
}

function dropLocal(id: string): void {
  useItemsStore.setState((s) => {
    const items = { ...s.items };
    delete items[id];
    return { items, order: s.order.filter((x) => x !== id) };
  });
}

/** Changes whenever the card (or its checklist / files) changes, to refresh sections. */
export function cardVersion(item: Item): string {
  return [
    item.updatedAt,
    item.subtasksDone ?? 0,
    item.subtasksTotal ?? 0,
    item.attachmentCount ?? 0,
  ].join(":");
}

/** Renames a project or personal board (blank or unchanged names are ignored). */
export async function renamePlace(
  kind: "project" | "board",
  place: { id: string; name: string },
  name: string,
): Promise<void> {
  const value = name.trim();
  if (!value || value === place.name) return;
  try {
    if (kind === "project") await cardsApi.renameProject(place.id, value);
    else await cardsApi.renameBoard(place.id, value);
  } catch (err) {
    toastError(message(err));
  }
}

/** Pins or unpins a card (pinned cards sort first). */
export async function setPinned(id: string, pinned: boolean): Promise<void> {
  const before = patchLocal(id, { pinned });
  try {
    const item = await cardsApi.pinItem(id, pinned);
    useItemsStore.setState((s) => ({ items: { ...s.items, [id]: item } }));
  } catch (err) {
    if (before) patchLocal(id, before);
    toastError(message(err));
  }
}

/** Archives a card, with Undo. */
export async function archiveCard(item: Item): Promise<void> {
  dropLocal(item.id);
  try {
    await cardsApi.archiveItem(item.id, true);
    useToastsStore.getState().push({
      message: cardsCopy.archived(item.title),
      action: {
        label: cardsCopy.undo,
        run: () => void cardsApi.archiveItem(item.id, false).catch((e) => toastError(message(e))),
      },
    });
  } catch (err) {
    toastError(message(err));
    void useItemsStore.getState().reload();
  }
}

/** Takes a card out of the Archive. */
export async function unarchiveCard(item: Item): Promise<void> {
  try {
    const restored = await cardsApi.archiveItem(item.id, false);
    useItemsStore.setState((s) => ({ items: { ...s.items, [item.id]: restored } }));
    useToastsStore.getState().push({ message: cardsCopy.restored(item.title) });
  } catch (err) {
    toastError(message(err));
  }
}

/** Takes a card out of the Trash. */
export async function restoreCard(item: Item): Promise<void> {
  try {
    const restored = await cardsApi.restoreItem(item.id);
    useItemsStore.setState((s) => ({ items: { ...s.items, [item.id]: restored } }));
    useToastsStore.getState().push({ message: cardsCopy.restored(item.title) });
  } catch (err) {
    toastError(message(err));
  }
}

// ── Confirm dialog ────────────────────────────────────────────────────────

export interface ConfirmRequest {
  title: string;
  body?: string;
  confirmLabel: string;
  danger?: boolean;
}

interface ConfirmState {
  request: (ConfirmRequest & { resolve: (ok: boolean) => void }) | null;
  answer: (ok: boolean) => void;
}

export const useConfirmStore = create<ConfirmState>()((set, get) => ({
  request: null,
  answer(ok) {
    const req = get().request;
    set({ request: null });
    req?.resolve(ok);
  },
}));

/** Asks the user to confirm a destructive action. Resolves `true` on confirm. */
export function confirmAction(request: ConfirmRequest): Promise<boolean> {
  return new Promise((resolve) => {
    useConfirmStore.getState().request?.resolve(false);
    useConfirmStore.setState({ request: { ...request, resolve } });
  });
}
