import { invoke } from "@tauri-apps/api/core";
import type { Counts, Item, ItemFilter, ItemPatch, NewItem } from "./types";

export function listItems(filter: ItemFilter): Promise<Item[]> {
  return invoke<Item[]>("items_list", { filter });
}

export function getItem(id: string): Promise<Item | null> {
  return invoke<Item | null>("items_get", { id });
}

export function createItem(input: NewItem): Promise<Item> {
  return invoke<Item>("items_create", { input });
}

export function updateItem(id: string, patch: ItemPatch): Promise<Item> {
  return invoke<Item>("items_update", { id, patch });
}

export function deleteItem(id: string): Promise<void> {
  return invoke<void>("items_delete", { id });
}

export function completeItem(id: string): Promise<Item> {
  return invoke<Item>("items_complete", { id });
}

export function getCounts(): Promise<Counts> {
  return invoke<Counts>("items_counts");
}

export function searchItems(query: string, limit = 20): Promise<Item[]> {
  return invoke<Item[]>("items_search", { query, limit });
}
