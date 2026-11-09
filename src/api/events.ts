import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ItemsChanged } from "./types";

export const ITEMS_CHANGED = "items:changed";

export function onItemsChanged(cb: (ids: string[]) => void): Promise<UnlistenFn> {
  return listen<ItemsChanged>(ITEMS_CHANGED, (event) => cb(event.payload.ids));
}
