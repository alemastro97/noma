import { invoke } from "@tauri-apps/api/core";
import type { Board, Link, Placement } from "./types";

export function listBoards(): Promise<Board[]> {
  return invoke<Board[]>("boards_list");
}

export function createBoard(name: string): Promise<Board> {
  return invoke<Board>("boards_create", { name });
}

export function listPlacements(boardId: string): Promise<Placement[]> {
  return invoke<Placement[]>("placements_list", { boardId });
}

export function setPlacement(
  itemId: string,
  boardId: string,
  x: number,
  y: number,
): Promise<Placement> {
  return invoke<Placement>("placements_set", { itemId, boardId, x, y });
}

export function listLinks(boardId: string): Promise<Link[]> {
  return invoke<Link[]>("links_list", { boardId });
}

export function createLink(fromItem: string, toItem: string): Promise<Link> {
  return invoke<Link>("links_create", { fromItem, toItem });
}

export function deleteLink(id: string): Promise<void> {
  return invoke<void>("links_delete", { id });
}
