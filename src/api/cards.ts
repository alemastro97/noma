import { invoke } from "@tauri-apps/api/core";
import type { Board, Item, Iso, Project } from "./types";

/** A checklist entry inside a card. */
export interface Subtask {
  id: string;
  itemId: string;
  title: string;
  done: boolean;
  position: number;
  createdAt: Iso;
}

/** A file attached to a card (stored in NOMA's data folder). */
export interface Attachment {
  id: string;
  itemId: string;
  name: string;
  file: string;
  mime: string;
  size: number;
  createdAt: Iso;
}

// ── Pin, archive, trash ──────────────────────────────────────────────────

export function pinItem(id: string, pinned: boolean): Promise<Item> {
  return invoke<Item>("items_pin", { id, pinned });
}

export function archiveItem(id: string, archived: boolean): Promise<Item> {
  return invoke<Item>("items_archive", { id, archived });
}

export function trashItem(id: string): Promise<Item> {
  return invoke<Item>("items_trash", { id });
}

export function restoreItem(id: string): Promise<Item> {
  return invoke<Item>("items_restore", { id });
}

export function listArchived(): Promise<Item[]> {
  return invoke<Item[]>("items_archived");
}

export function listTrashed(): Promise<Item[]> {
  return invoke<Item[]>("items_trashed");
}

export function deleteForever(id: string): Promise<void> {
  return invoke<void>("items_delete_forever", { id });
}

export function emptyTrash(): Promise<number> {
  return invoke<number>("trash_empty");
}

// ── Subtasks ─────────────────────────────────────────────────────────────

export function listSubtasks(itemId: string): Promise<Subtask[]> {
  return invoke<Subtask[]>("subtasks_list", { itemId });
}

export function addSubtask(itemId: string, title: string): Promise<Subtask> {
  return invoke<Subtask>("subtasks_add", { itemId, title });
}

export function updateSubtask(
  id: string,
  patch: { title?: string; done?: boolean },
): Promise<Subtask> {
  return invoke<Subtask>("subtasks_update", { id, title: patch.title, done: patch.done });
}

export function deleteSubtask(id: string): Promise<void> {
  return invoke<void>("subtasks_delete", { id });
}

export function reorderSubtasks(itemId: string, ids: string[]): Promise<void> {
  return invoke<void>("subtasks_reorder", { itemId, ids });
}

// ── Attachments ──────────────────────────────────────────────────────────

export function listAttachments(itemId: string): Promise<Attachment[]> {
  return invoke<Attachment[]>("attachments_list", { itemId });
}

function toBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}

export async function addAttachment(itemId: string, file: File): Promise<Attachment> {
  const data = toBase64(new Uint8Array(await file.arrayBuffer()));
  return invoke<Attachment>("attachments_add", { itemId, name: file.name, data });
}

/** The file's bytes, for image thumbnails. */
export function readAttachment(id: string): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>("attachments_read", { id });
}

export function openAttachment(id: string, reveal = false): Promise<void> {
  return invoke<void>("attachments_open", { id, reveal });
}

export function deleteAttachment(id: string): Promise<void> {
  return invoke<void>("attachments_delete", { id });
}

/** Opens a web or mail link from a card's notes in the default browser. */
export function openLink(url: string): Promise<void> {
  return invoke<void>("card_open_link", { url });
}

// ── Projects and boards ──────────────────────────────────────────────────

export function renameProject(id: string, name: string): Promise<Project> {
  return invoke<Project>("projects_rename", { id, name });
}

export function setProjectColor(id: string, color: string): Promise<Project> {
  return invoke<Project>("projects_set_color", { id, color });
}

export function archiveProject(id: string, archived: boolean): Promise<Project> {
  return invoke<Project>("projects_archive", { id, archived });
}

/** Deletes a project; its cards move to the Trash. Returns how many. */
export function deleteProject(id: string): Promise<number> {
  return invoke<number>("projects_delete", { id });
}

export function listArchivedProjects(): Promise<Project[]> {
  return invoke<Project[]>("projects_archived");
}

export function renameBoard(id: string, name: string): Promise<Board> {
  return invoke<Board>("boards_rename", { id, name });
}

export function archiveBoard(id: string, archived: boolean): Promise<Board> {
  return invoke<Board>("boards_archive", { id, archived });
}

/** Deletes a personal board; its own cards move to the Trash. Returns how many. */
export function deleteBoard(id: string): Promise<number> {
  return invoke<number>("boards_delete", { id });
}

export function listArchivedBoards(): Promise<Board[]> {
  return invoke<Board[]>("boards_archived");
}
