/**
 * In-memory stand-in for the Rust backend, used only by `pnpm dev` in a plain
 * browser (no Tauri). It lets the UI be developed and checked visually against
 * the mockups without the desktop shell. Never bundled into the app.
 */
import { mockIPC } from "@tauri-apps/api/mocks";
import type {
  Board,
  Counts,
  Item,
  ItemPatch,
  Link,
  NewItem,
  Progress,
  Project,
} from "../api/types";

type Args = Record<string, unknown>;

const items = new Map<string, Item>();
const projects: Project[] = [];
const boards: Board[] = [];
const placements = new Map<string, { boardId: string; x: number; y: number }>();
let links: Link[] = [];
let seq = 0;

const id = () => `m${++seq}`;
const iso = (d: Date) => d.toISOString();
/** Event name → registered Tauri callback ids. */
const listeners = new Map<string, Set<number>>();

function emit(event: string, payload: unknown) {
  const internals = (
    window as unknown as {
      __TAURI_INTERNALS__: { runCallback: (id: number, data: unknown) => void };
    }
  ).__TAURI_INTERNALS__;
  for (const id of listeners.get(event) ?? []) internals.runCallback(id, { event, id: 0, payload });
}

function at(days: number, h: number, m = 0): string {
  const d = new Date();
  d.setDate(d.getDate() + days);
  d.setHours(h, m, 0, 0);
  return iso(d);
}

function daysUntil(weekday: number): number {
  return (weekday - new Date().getDay() + 7) % 7;
}

function create(input: NewItem): Item {
  const now = iso(new Date());
  const item: Item = {
    id: id(),
    kind: input.kind,
    title: input.title.trim(),
    body: input.body ?? null,
    status: input.status ?? "inbox",
    priority: input.priority ?? "normal",
    dueAt: input.dueAt ?? null,
    endsAt: input.endsAt ?? null,
    remindAt: input.remindAt ?? null,
    snoozedUntil: null,
    recurrence: input.recurrence ?? null,
    projectId: input.projectId ?? null,
    createdAt: now,
    updatedAt: now,
    completedAt: input.status === "done" ? now : null,
    tags: input.tags ?? [],
    linkCount: 0,
    hint: null,
    capturing: false,
  };
  items.set(item.id, item);
  const boardId = input.boardId ?? projects.find((p) => p.id === input.projectId)?.boardId ?? null;
  if (boardId) {
    const taken = [...placements.values()].filter((p) => p.boardId === boardId);
    let slot = { x: 80, y: 80 };
    for (let n = 0; ; n++) {
      slot = { x: 80 + (n % 4) * 300, y: 80 + Math.floor(n / 4) * 200 };
      if (taken.every((t) => Math.abs(t.x - slot.x) >= 280 || Math.abs(t.y - slot.y) >= 180)) break;
    }
    placements.set(item.id, { boardId, x: input.x ?? slot.x, y: input.y ?? slot.y });
  }
  return item;
}

function withLinks(item: Item): Item {
  return {
    ...item,
    capturing: capturing.has(item.id),
    linkCount: links.filter((l) => l.fromItem === item.id || l.toItem === item.id).length,
  };
}

function dayBounds(): [number, number] {
  const s = new Date();
  s.setHours(0, 0, 0, 0);
  return [s.getTime(), s.getTime() + 86_400_000];
}

function list(filter: { kind: string; id?: string; days?: number }, includeDone: boolean): Item[] {
  const all = [...items.values()].map(withLinks);
  const open = (i: Item) => includeDone || i.status !== "done";
  const t = (s: string | null) => (s ? new Date(s).getTime() : NaN);
  const [start, end] = dayBounds();
  let out: Item[];
  switch (filter.kind) {
    case "inbox":
      out = all.filter(
        (i) => open(i) && i.kind !== "project_ref" && !i.projectId && !placements.has(i.id),
      );
      break;
    case "today":
      out = all.filter(
        (i) =>
          i.kind !== "project_ref" &&
          ((i.status !== "done" && (t(i.dueAt) < end || t(i.remindAt) < end)) ||
            (includeDone && i.status === "done" && t(i.completedAt) >= start)),
      );
      break;
    case "upcoming": {
      const until = end + (filter.days ?? 7) * 86_400_000;
      const inRange = (v: number) => v >= end && v < until;
      out = all.filter(
        (i) =>
          open(i) && i.kind !== "project_ref" && (inRange(t(i.dueAt)) || inRange(t(i.remindAt))),
      );
      break;
    }
    case "ideas":
      out = all.filter((i) => open(i) && i.kind === "idea");
      break;
    case "notes":
      out = all.filter((i) => open(i) && i.kind === "note");
      break;
    case "project":
      out = all.filter((i) => open(i) && i.projectId === filter.id);
      break;
    case "board":
      out = all.filter((i) => open(i) && placements.get(i.id)?.boardId === filter.id);
      break;
    default:
      out = all.filter(open);
  }
  return out.sort((a, b) => {
    const da = t(a.dueAt ?? a.remindAt);
    const db = t(b.dueAt ?? b.remindAt);
    return (Number.isNaN(da) ? Infinity : da) - (Number.isNaN(db) ? Infinity : db);
  });
}

function counts(): Counts {
  const byProject: Record<string, number> = {};
  for (const i of items.values())
    if (i.projectId && i.status !== "done" && i.kind !== "project_ref")
      byProject[i.projectId] = (byProject[i.projectId] ?? 0) + 1;
  return {
    inbox: list({ kind: "inbox" }, false).length,
    today: list({ kind: "today" }, false).length,
    upcoming: list({ kind: "upcoming" }, false).length,
    ideas: list({ kind: "ideas" }, false).length,
    notes: list({ kind: "notes" }, false).length,
    byProject,
  };
}

function progress(): Record<string, Progress> {
  const out: Record<string, Progress> = {};
  for (const i of items.values()) {
    if (!i.projectId || i.kind === "project_ref") continue;
    const p = (out[i.projectId] ??= { done: 0, total: 0 });
    p.total++;
    if (i.status === "done") p.done++;
  }
  return out;
}

function createProject(name: string, color: string): Project {
  const board: Board = { id: id(), name, kind: "project", createdAt: iso(new Date()) };
  boards.push(board);
  const project: Project = { id: id(), name, color, boardId: board.id, createdAt: board.createdAt };
  projects.push(project);
  return project;
}

function seed(): boolean {
  if (items.size > 0) return false;
  const spring = createProject("Spring launch", "#79B8F5");
  createProject("Studio move", "#78DCC0");
  createProject("Reading list", "#F2BD73");
  for (const name of ["Thinking space", "Trips 2027"])
    boards.push({ id: id(), name, kind: "personal", createdAt: iso(new Date()) });
  const on = (x: number, y: number) => ({ projectId: spring.id, x, y });
  const project = create({
    kind: "project_ref",
    title: "Spring launch",
    body: "Site, deck and press kit for the April release.",
    ...on(80, 110),
  });
  const deck = create({
    kind: "task",
    title: "Send revised deck to Marta",
    priority: "high",
    dueAt: at(daysUntil(4), 18),
    ...on(520, 60),
  });
  const printer = create({
    kind: "reminder",
    title: "Call the printer about proofs",
    remindAt: at(0, 16),
    ...on(520, 290),
  });
  const notes = create({
    kind: "note",
    title: "Interview notes — Giulia",
    body: "She keeps reminders in three apps and trusts none of them. “I just want to say it once and forget about it.”",
    tags: ["research", "voice"],
    ...on(110, 470),
  });
  const idea = create({
    kind: "idea",
    title: "What if the orb dims while you’re in a meeting?",
    tags: ["product"],
    ...on(860, 110),
  });
  const review = create({
    kind: "event",
    title: "Design review",
    dueAt: at(daysUntil(3), 14, 30),
    endsAt: at(daysUntil(3), 15, 30),
    ...on(860, 300),
  });
  const photo = create({
    kind: "task",
    title: "Book photographer for launch shoot",
    dueAt: at(Math.max(daysUntil(1), 1), 10),
    remindAt: at(daysUntil(5), 9),
    ...on(600, 470),
  });
  for (const title of ["Review the press kit copy", "Shortlist three venues", "Draft launch email"])
    create({ kind: "task", title, projectId: spring.id });
  const done = create({ kind: "task", title: "Find a good product name", projectId: spring.id });
  done.status = "done";
  done.completedAt = iso(new Date());
  for (const [a, b] of [
    [project, deck],
    [project, printer],
    [project, notes],
    [review, photo],
    [idea, notes],
  ] as const)
    links.push({ id: id(), fromItem: a.id, toItem: b.id, label: null, createdAt: iso(new Date()) });
  create({ kind: "task", title: "Pick up the studio keys", dueAt: at(0, 18) });
  create({
    kind: "reminder",
    title: "Weekly review",
    remindAt: at(daysUntil(1), 9),
    recurrence: "FREQ=WEEKLY;BYDAY=MO",
  });
  create({ kind: "idea", title: "A calmer notification sound" });
  return true;
}

function changed() {
  emit("items:changed", { ids: [] });
}

// ── Simulated AI: a keyword heuristic, enough to exercise the capture UI. ──
const aiState = {
  provider: "builtin",
  modelId: "qwen3-4b-instruct-2507-q4km",
  ollamaModel: null,
  ollamaUrl: "http://127.0.0.1:11434",
  ready: true,
  phase: "idle",
  message: null,
  serverInstalled: true,
  serverRunning: false,
  models: [
    {
      id: "qwen3-4b-instruct-2507-q4km",
      name: "Qwen3 4B Instruct",
      description: "Best balance of speed and accuracy. Italian and English.",
      file: "Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
      sizeBytes: 2497281120,
      ramGb: 3.5,
      recommended: true,
      installed: true,
    },
    {
      id: "qwen3-1.7b-q4km",
      name: "Qwen3 1.7B",
      description: "Lighter and faster, for machines with less memory.",
      file: "Qwen3-1.7B-Q4_K_M.gguf",
      sizeBytes: 1107409472,
      ramGb: 2,
      recommended: false,
      installed: false,
    },
  ],
  download: null as null | { modelId: string; stage: string; received: number; total: number },
  ollamaAvailable: false,
  ollamaModels: [] as string[],
  pending: 0,
};
const capturing = new Set<string>();

function publishAi() {
  emit("ai:state", { ...aiState, pending: capturing.size });
}

function fakeExtract(text: string): Partial<Item> {
  const t = text.toLowerCase();
  const when = (days: number, h: number) => at(days, h);
  const tomorrow = /domani|tomorrow/.test(t);
  if (/ricordami|remind/.test(t))
    return { kind: "reminder", remindAt: when(tomorrow ? 1 : 0, tomorrow ? 9 : 18) };
  if (/idea|what if|e se/.test(t)) return { kind: "idea" };
  if (/meeting|riunione|call with|pranzo|dinner|cena/.test(t))
    return { kind: "event", dueAt: when(1, 14), endsAt: when(1, 15) };
  if (/password|codice|note/.test(t)) return { kind: "note" };
  return { kind: "task", dueAt: tomorrow ? when(1, 0) : null };
}

function simulateCapture(itemId: string, text: string) {
  capturing.add(itemId);
  aiState.phase = "processing";
  publishAi();
  setTimeout(() => {
    capturing.delete(itemId);
    const item = items.get(itemId);
    if (item) {
      const title = text
        .replace(/^(ricordami di|ricordami|remind me to|idea:?)\s*/i, "")
        .replace(/\s*(domani|tomorrow)\b.*$/i, "")
        .trim();
      items.set(itemId, {
        ...item,
        ...fakeExtract(text),
        title: title.charAt(0).toUpperCase() + title.slice(1) || item.title,
        capturing: false,
        hint: /venerd|friday/.test(text.toLowerCase()) ? { datePhrase: "friday" } : null,
      });
    }
    aiState.phase = "idle";
    changed();
    emit("ai:capture", {
      captureId: itemId,
      status: "done",
      itemIds: [itemId],
      error: null,
      latencyMs: 840,
    });
    publishAi();
  }, 900);
}

function handle(cmd: string, a: Args): unknown {
  switch (cmd) {
    case "items_list":
      return list(a.filter as { kind: string }, Boolean(a.includeDone));
    case "items_get":
      return items.has(a.id as string) ? withLinks(items.get(a.id as string)!) : null;
    case "items_create": {
      const item = create(a.input as NewItem);
      changed();
      return item;
    }
    case "items_update": {
      const item = items.get(a.id as string);
      if (!item) throw new Error("item not found");
      const patch = a.patch as ItemPatch;
      if (patch.status === "done") return handle("items_complete", a);
      const next: Item = {
        ...item,
        ...patch,
        tags: patch.tags ?? item.tags,
        updatedAt: iso(new Date()),
      };
      if (patch.status) next.completedAt = null;
      items.set(next.id, next);
      changed();
      return withLinks(next);
    }
    case "items_complete": {
      const item = items.get(a.id as string)!;
      if (item.recurrence && (item.dueAt || item.remindAt)) {
        const shift = (s: string | null) =>
          s ? iso(new Date(new Date(s).getTime() + 7 * 86_400_000)) : null;
        const next = { ...item, dueAt: shift(item.dueAt), remindAt: shift(item.remindAt) };
        items.set(next.id, next);
        changed();
        return next;
      }
      const next: Item = { ...item, status: "done", completedAt: iso(new Date()) };
      items.set(next.id, next);
      changed();
      return next;
    }
    case "items_delete":
      items.delete(a.id as string);
      placements.delete(a.id as string);
      links = links.filter((l) => l.fromItem !== a.id && l.toItem !== a.id);
      changed();
      return null;
    case "items_duplicate": {
      const src = items.get(a.id as string)!;
      const p = placements.get(src.id);
      const copy = create({
        ...src,
        boardId: p?.boardId,
        x: p ? p.x + 24 : null,
        y: p ? p.y + 24 : null,
        status: null,
      });
      changed();
      return copy;
    }
    case "items_counts":
      return counts();
    case "items_search": {
      const q = String(a.query).toLowerCase();
      return [...items.values()]
        .filter((i) => `${i.title} ${i.body ?? ""}`.toLowerCase().includes(q))
        .slice(0, Number(a.limit ?? 20));
    }
    case "tags_list":
      return [...new Set([...items.values()].flatMap((i) => i.tags))];
    case "dev_seed": {
      const ok = seed();
      changed();
      return ok;
    }
    case "projects_list":
      return projects;
    case "projects_create": {
      const p = createProject(String(a.name), String(a.color));
      changed();
      return p;
    }
    case "projects_progress":
      return progress();
    case "boards_list":
      return boards;
    case "boards_create": {
      const b: Board = {
        id: id(),
        name: String(a.name),
        kind: "personal",
        createdAt: iso(new Date()),
      };
      boards.push(b);
      changed();
      return b;
    }
    case "placements_list":
      return [...placements.entries()]
        .filter(([, p]) => p.boardId === a.boardId)
        .map(([itemId, p]) => ({ itemId, ...p }));
    case "placements_set":
      placements.set(a.itemId as string, {
        boardId: a.boardId as string,
        x: a.x as number,
        y: a.y as number,
      });
      return { itemId: a.itemId, boardId: a.boardId, x: a.x, y: a.y };
    case "links_list":
      return links.filter(
        (l) =>
          placements.get(l.fromItem)?.boardId === a.boardId &&
          placements.get(l.toItem)?.boardId === a.boardId,
      );
    case "links_create": {
      const existing = links.find(
        (l) =>
          (l.fromItem === a.fromItem && l.toItem === a.toItem) ||
          (l.fromItem === a.toItem && l.toItem === a.fromItem),
      );
      if (existing) return existing;
      const l: Link = {
        id: id(),
        fromItem: a.fromItem as string,
        toItem: a.toItem as string,
        label: null,
        createdAt: iso(new Date()),
      };
      links.push(l);
      changed();
      return l;
    }
    case "ai_state":
    case "ai_refresh":
      return { ...aiState, pending: capturing.size };
    case "ai_configure":
      Object.assign(aiState, a.patch as object);
      publishAi();
      return { ...aiState, pending: capturing.size };
    case "ai_download": {
      const model = aiState.models.find((m) => m.id === a.modelId);
      if (!model) return null;
      let received = 0;
      const tick = setInterval(() => {
        received = Math.min(model.sizeBytes, received + model.sizeBytes / 12);
        aiState.download = { modelId: model.id, stage: "model", received, total: model.sizeBytes };
        if (received >= model.sizeBytes) {
          clearInterval(tick);
          aiState.download = null;
          model.installed = true;
          aiState.modelId = model.id;
        }
        publishAi();
      }, 250);
      return null;
    }
    case "ai_cancel_download":
      aiState.download = null;
      publishAi();
      return null;
    case "ai_remove_model": {
      const model = aiState.models.find((m) => m.id === a.modelId);
      if (model) model.installed = false;
      aiState.ready = aiState.models.some((m) => m.installed && m.id === aiState.modelId);
      publishAi();
      return { ...aiState, pending: capturing.size };
    }
    case "capture_text": {
      const text = String(a.text).trim();
      const ctx = (a.context ?? {}) as {
        projectId?: string;
        boardId?: string;
        x?: number;
        y?: number;
        status?: Item["status"];
      };
      const item = create({
        kind: "note",
        title: text.split("\n")[0]!.slice(0, 120),
        projectId: ctx.projectId ?? null,
        boardId: ctx.boardId ?? null,
        x: ctx.x ?? null,
        y: ctx.y ?? null,
        status: ctx.status ?? null,
      });
      simulateCapture(item.id, text);
      changed();
      return { captureId: item.id, item: withLinks(item) };
    }
    case "capture_retry":
      return null;
    case "links_delete":
      links = links.filter((l) => l.id !== a.id);
      changed();
      return null;
    default:
      console.warn(`[mock backend] unhandled command ${cmd}`);
      return null;
  }
}

export function installMockBackend(): void {
  seed();
  mockIPC((cmd, args) => {
    if (cmd === "plugin:event|listen") {
      const { event, handler } = args as { event: string; handler: number };
      if (!listeners.has(event)) listeners.set(event, new Set());
      listeners.get(event)!.add(handler);
      return handler;
    }
    if (cmd.startsWith("plugin:")) return null;
    return handle(cmd, (args ?? {}) as Args);
  });
}
