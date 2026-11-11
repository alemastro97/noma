/** Mock backend for the cards area (pin, archive, trash, subtasks, attachments, places). */
import type { Attachment, Subtask } from "../../api/cards";
import type { Board, Item, Project } from "../../api/types";
import { mockState } from "../mockBackend";

export const NOT_HANDLED = Symbol("not handled");

type Args = Record<string, unknown>;

const subtasks = new Map<string, Subtask>();
const attachments = new Map<string, Attachment & { data: Uint8Array }>();
const archivedProjects: (Project & { archivedAt: string })[] = [];
const archivedBoards: (Board & { archivedAt: string })[] = [];

function item(id: unknown): Item {
  const found = mockState.items.get(String(id));
  if (!found) throw new Error("item not found");
  return found;
}

function withCounts(i: Item): Item {
  const subs = [...subtasks.values()].filter((s) => s.itemId === i.id);
  return {
    ...mockState.withLinks(i),
    subtasksTotal: subs.length,
    subtasksDone: subs.filter((s) => s.done).length,
    attachmentCount: [...attachments.values()].filter((a) => a.itemId === i.id).length,
  };
}

function patch(id: unknown, fields: Partial<Item>): Item {
  const next = { ...item(id), ...fields };
  mockState.items.set(next.id, next);
  mockState.changed();
  return withCounts(next);
}

function hardDelete(ids: string[]): void {
  for (const id of ids) {
    mockState.items.delete(id);
    mockState.placements.delete(id);
    for (const [k, s] of subtasks) if (s.itemId === id) subtasks.delete(k);
    for (const [k, a] of attachments) if (a.itemId === id) attachments.delete(k);
  }
  mockState.changed();
}

function meta(a: Attachment): Attachment {
  return {
    id: a.id,
    itemId: a.itemId,
    name: a.name,
    file: a.file,
    mime: a.mime,
    size: a.size,
    createdAt: a.createdAt,
  };
}

function itemSubtasks(itemId: string): Subtask[] {
  return [...subtasks.values()]
    .filter((s) => s.itemId === itemId)
    .sort((a, b) => a.position - b.position);
}

function fromBase64(data: string): Uint8Array {
  const bin = atob(data);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

function mimeFor(name: string): string {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  const images: Record<string, string> = {
    png: "image/png",
    jpg: "image/jpeg",
    jpeg: "image/jpeg",
    gif: "image/gif",
    webp: "image/webp",
    svg: "image/svg+xml",
  };
  return images[ext] ?? (ext === "pdf" ? "application/pdf" : "application/octet-stream");
}

function trashPlaceItems(match: (i: Item) => boolean): number {
  const now = mockState.iso(new Date());
  let n = 0;
  for (const i of mockState.items.values()) {
    if (!i.deletedAt && match(i)) {
      mockState.items.set(i.id, { ...i, deletedAt: now, projectId: null });
      n++;
    }
  }
  return n;
}

export function handleCards(cmd: string, a: Args): unknown {
  const now = () => mockState.iso(new Date());
  switch (cmd) {
    case "items_pin":
      return patch(a.id, { pinned: Boolean(a.pinned) });
    case "items_archive":
      return patch(a.id, { archivedAt: a.archived ? now() : null });
    case "items_trash":
      return patch(a.id, { deletedAt: now() });
    case "items_restore":
      return patch(a.id, { deletedAt: null });
    case "items_archived":
      return [...mockState.items.values()]
        .filter((i) => i.archivedAt && !i.deletedAt)
        .map(withCounts);
    case "items_trashed":
      return [...mockState.items.values()].filter((i) => i.deletedAt).map(withCounts);
    case "items_delete_forever":
      hardDelete([String(a.id)]);
      return null;
    case "trash_empty": {
      const ids = [...mockState.items.values()].filter((i) => i.deletedAt).map((i) => i.id);
      hardDelete(ids);
      return ids.length;
    }

    case "subtasks_list":
      return itemSubtasks(String(a.itemId));
    case "subtasks_add": {
      const itemId = String(a.itemId);
      const title = String(a.title).trim();
      if (!title) throw new Error("subtask must not be blank");
      const s: Subtask = {
        id: mockState.id(),
        itemId,
        title,
        done: false,
        position: itemSubtasks(itemId).length,
        createdAt: now(),
      };
      subtasks.set(s.id, s);
      mockState.changed();
      return s;
    }
    case "subtasks_update": {
      const s = subtasks.get(String(a.id));
      if (!s) throw new Error("subtask not found");
      const next = {
        ...s,
        title: typeof a.title === "string" ? a.title.trim() || s.title : s.title,
        done: typeof a.done === "boolean" ? a.done : s.done,
      };
      subtasks.set(s.id, next);
      mockState.changed();
      return next;
    }
    case "subtasks_delete":
      subtasks.delete(String(a.id));
      mockState.changed();
      return null;
    case "subtasks_reorder":
      (a.ids as string[]).forEach((id, position) => {
        const s = subtasks.get(id);
        if (s) subtasks.set(id, { ...s, position });
      });
      mockState.changed();
      return null;

    case "attachments_list":
      return [...attachments.values()].filter((x) => x.itemId === a.itemId).map(meta);
    case "attachments_add": {
      const data = fromBase64(String(a.data));
      const name = String(a.name);
      const att = {
        id: mockState.id(),
        itemId: String(a.itemId),
        name,
        file: name,
        mime: mimeFor(name),
        size: data.length,
        createdAt: now(),
        data,
      };
      attachments.set(att.id, att);
      mockState.changed();
      return meta(att);
    }
    case "attachments_read": {
      const att = attachments.get(String(a.id));
      if (!att) throw new Error("attachment not found");
      return att.data.slice().buffer;
    }
    case "attachments_open":
    case "card_open_link":
      if (cmd === "card_open_link") window.open(String(a.url), "_blank", "noopener");
      return null;
    case "attachments_delete":
      attachments.delete(String(a.id));
      mockState.changed();
      return null;

    case "projects_rename":
    case "projects_set_color": {
      const p = mockState.projects.find((x) => x.id === a.id);
      if (!p) throw new Error("project not found");
      if (cmd === "projects_rename") p.name = String(a.name).trim() || p.name;
      else p.color = String(a.color);
      mockState.changed();
      return p;
    }
    case "projects_archive": {
      if (a.archived) {
        const index = mockState.projects.findIndex((x) => x.id === a.id);
        if (index < 0) throw new Error("project not found");
        const [p] = mockState.projects.splice(index, 1);
        archivedProjects.unshift({ ...p!, archivedAt: now() });
        mockState.changed();
        return p;
      }
      const index = archivedProjects.findIndex((x) => x.id === a.id);
      if (index < 0) throw new Error("project not found");
      const [archived] = archivedProjects.splice(index, 1);
      const p: Project = {
        id: archived!.id,
        name: archived!.name,
        color: archived!.color,
        boardId: archived!.boardId,
        createdAt: archived!.createdAt,
      };
      mockState.projects.push(p);
      mockState.changed();
      return p;
    }
    case "projects_delete": {
      const index = mockState.projects.findIndex((x) => x.id === a.id);
      if (index < 0) throw new Error("project not found");
      const [p] = mockState.projects.splice(index, 1);
      const n = trashPlaceItems((i) => i.projectId === p!.id);
      mockState.changed();
      return n;
    }
    case "projects_archived":
      return archivedProjects;

    case "boards_rename": {
      const b = mockState.boards.find((x) => x.id === a.id);
      if (!b) throw new Error("board not found");
      b.name = String(a.name).trim() || b.name;
      mockState.changed();
      return b;
    }
    case "boards_archive": {
      if (a.archived) {
        const index = mockState.boards.findIndex((x) => x.id === a.id);
        if (index < 0) throw new Error("board not found");
        const [b] = mockState.boards.splice(index, 1);
        archivedBoards.unshift({ ...b!, archivedAt: now() });
        mockState.changed();
        return b;
      }
      const index = archivedBoards.findIndex((x) => x.id === a.id);
      if (index < 0) throw new Error("board not found");
      const [archived] = archivedBoards.splice(index, 1);
      const b: Board = {
        id: archived!.id,
        name: archived!.name,
        kind: archived!.kind,
        createdAt: archived!.createdAt,
      };
      mockState.boards.push(b);
      mockState.changed();
      return b;
    }
    case "boards_delete": {
      const index = mockState.boards.findIndex((x) => x.id === a.id);
      if (index < 0) throw new Error("board not found");
      const [b] = mockState.boards.splice(index, 1);
      const n = trashPlaceItems(
        (i) => !i.projectId && mockState.placements.get(i.id)?.boardId === b!.id,
      );
      for (const [itemId, p] of mockState.placements)
        if (p.boardId === b!.id) mockState.placements.delete(itemId);
      mockState.changed();
      return n;
    }
    case "boards_archived":
      return archivedBoards;

    default:
      return NOT_HANDLED;
  }
}
