/**
 * Mock backend cases for the workspace area (saved filters, canvas groups, the
 * links graph, bulk moves, delete snapshots). Browser dev mode only.
 */
import type { Item, Link, Place, Project } from "../../api/types";
import type { CanvasGroup, PriorPlace, SavedFilter } from "../../api/workspace";

type Args = Record<string, unknown>;
type Placement = { boardId: string; x: number; y: number };

/** The mock backend's state, handed in so this file can share it. */
export interface WorkspaceMockContext {
  items: Map<string, Item>;
  projects: Project[];
  placements: Map<string, Placement>;
  getLinks: () => Link[];
  setLinks: (links: Link[]) => void;
  newId: () => string;
  changed: () => void;
}

interface MockSnapshot {
  item: Item;
  placement: Placement | null;
  links: Link[];
}

const filters: SavedFilter[] = [];
const groups: CanvasGroup[] = [];

function placeOf(ctx: WorkspaceMockContext, id: string): Place {
  const item = ctx.items.get(id);
  if (item?.projectId) return { kind: "project", id: item.projectId };
  const p = ctx.placements.get(id);
  const isProjectBoard = p && ctx.projects.some((x) => x.boardId === p.boardId);
  return p && !isProjectBoard ? { kind: "board", id: p.boardId } : { kind: "inbox" };
}

function moveTo(ctx: WorkspaceMockContext, id: string, place: Place) {
  const item = ctx.items.get(id);
  if (!item) return;
  if (place.kind === "project") {
    const project = ctx.projects.find((p) => p.id === place.id);
    ctx.items.set(id, { ...item, projectId: place.id });
    if (project) ctx.placements.set(id, { boardId: project.boardId, x: 80, y: 80 });
  } else if (place.kind === "board") {
    ctx.items.set(id, { ...item, projectId: null });
    ctx.placements.set(id, { boardId: place.id, x: 80 + Math.random() * 600, y: 80 });
  } else {
    ctx.items.set(id, { ...item, projectId: null });
    ctx.placements.delete(id);
  }
}

/** Returns `undefined` when `cmd` is not a workspace command. */
export function handleWorkspace(cmd: string, a: Args, ctx: WorkspaceMockContext): unknown {
  switch (cmd) {
    case "filters_list":
      return filters;
    case "filters_create": {
      const f: SavedFilter = {
        id: ctx.newId(),
        name: String(a.name).trim(),
        spec: (a.spec ?? {}) as SavedFilter["spec"],
        createdAt: new Date().toISOString(),
      };
      filters.push(f);
      return f;
    }
    case "filters_delete": {
      const i = filters.findIndex((f) => f.id === a.id);
      if (i >= 0) filters.splice(i, 1);
      return null;
    }
    case "groups_list":
      return groups.filter((g) => g.boardId === a.boardId);
    case "groups_create": {
      const g: CanvasGroup = {
        id: ctx.newId(),
        boardId: String(a.boardId),
        label: String(a.label),
        x: Number(a.x),
        y: Number(a.y),
        w: Math.max(80, Number(a.w)),
        h: Math.max(80, Number(a.h)),
      };
      groups.push(g);
      return g;
    }
    case "groups_update": {
      const i = groups.findIndex((g) => g.id === a.id);
      if (i < 0) throw new Error("group not found");
      groups[i] = { ...groups[i]!, ...(a.patch as Partial<CanvasGroup>) };
      return groups[i];
    }
    case "groups_delete": {
      const i = groups.findIndex((g) => g.id === a.id);
      if (i >= 0) groups.splice(i, 1);
      return null;
    }
    case "links_all":
      return ctx.getLinks();
    case "items_move": {
      const ids = a.ids as string[];
      const place = a.place as Place;
      const prior: PriorPlace[] = ids.map((id) => {
        const p = ctx.placements.get(id);
        return {
          id,
          place: placeOf(ctx, id),
          boardId: p?.boardId ?? null,
          x: p?.x ?? null,
          y: p?.y ?? null,
        };
      });
      for (const id of ids) moveTo(ctx, id, place);
      ctx.changed();
      return prior;
    }
    case "items_unmove": {
      for (const p of a.prior as PriorPlace[]) {
        moveTo(ctx, p.id, p.place);
        if (p.boardId && p.x !== null && p.y !== null)
          ctx.placements.set(p.id, { boardId: p.boardId, x: p.x, y: p.y });
      }
      ctx.changed();
      return null;
    }
    case "items_snapshot":
      return (a.ids as string[])
        .filter((id) => ctx.items.has(id))
        .map((id): MockSnapshot => ({
          item: ctx.items.get(id)!,
          placement: ctx.placements.get(id) ?? null,
          links: ctx.getLinks().filter((l) => l.fromItem === id || l.toItem === id),
        }));
    case "items_restore": {
      const snaps = a.snapshots as MockSnapshot[];
      const restored: string[] = [];
      for (const s of snaps) {
        if (ctx.items.has(s.item.id)) continue;
        ctx.items.set(s.item.id, s.item);
        if (s.placement) ctx.placements.set(s.item.id, s.placement);
        restored.push(s.item.id);
      }
      const links = ctx.getLinks();
      for (const s of snaps)
        for (const l of s.links)
          if (
            ctx.items.has(l.fromItem) &&
            ctx.items.has(l.toItem) &&
            !links.some((x) => x.id === l.id)
          )
            links.push(l);
      ctx.setLinks(links);
      ctx.changed();
      return restored;
    }
    default:
      return undefined;
  }
}
