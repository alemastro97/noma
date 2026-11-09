import { invoke } from "@tauri-apps/api/core";
import type { Board, Placement } from "./types";

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
