/**
 * Mock organize commands for `pnpm dev` in a plain browser. Built on the other
 * mock commands (through `invoke`), so it needs nothing from mockBackend's state.
 */
import { invoke } from "@tauri-apps/api/core";
import type { ProjectBrief, Review } from "../../api/organize";
import type { Item } from "../../api/types";

export const NOT_HANDLED = Symbol("not handled");

const briefs = new Map<string, ProjectBrief>();
const DAY = 86_400_000;

const allItems = () => invoke<Item[]>("items_list", { filter: { kind: "all" }, includeDone: true });
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

function brief(projectId: string): ProjectBrief {
  return (
    briefs.get(projectId) ?? {
      projectId,
      summary: null,
      summaryAt: null,
      nextAction: null,
      nextItemId: null,
      nextAt: null,
    }
  );
}

async function review(): Promise<Review> {
  const now = Date.now();
  const items = await allItems();
  const open = items.filter((i) => i.status !== "done" && i.kind !== "project_ref");
  const due = (i: Item) => Date.parse(i.dueAt ?? i.remindAt ?? "") || null;
  return {
    completed: items.filter(
      (i) => i.status === "done" && i.completedAt && Date.parse(i.completedAt) > now - 7 * DAY,
    ),
    overdue: open.filter(
      (i) => (i.kind === "task" || i.kind === "reminder") && (due(i) ?? Infinity) < now,
    ),
    forgotten: open.filter((i) => Date.parse(i.updatedAt) < now - 30 * DAY),
  };
}

async function organize(cmd: string, a: Record<string, unknown>): Promise<unknown> {
  switch (cmd) {
    case "organize_resort_inbox":
      await wait(900);
      return { checked: 0, moved: 0 };
    case "organize_merge": {
      const [dup, into] = [a.duplicate as string, a.into as string];
      const items = await allItems();
      const d = items.find((i) => i.id === dup);
      const t = items.find((i) => i.id === into);
      if (!d || !t) throw new Error("card not found");
      const body = [t.body, d.title].filter(Boolean).join("\n\n");
      const merged = await invoke<Item>("items_update", { id: into, patch: { body } });
      await invoke("items_delete", { id: dup });
      return merged;
    }
    case "organize_break_down": {
      await wait(900);
      const parent = (await allItems()).find((i) => i.id === a.itemId);
      if (!parent) throw new Error("card not found");
      const ids: string[] = [];
      for (const title of ["Outline the first step", "Do the main part", "Check and wrap up"]) {
        const step = await invoke<Item>("items_create", {
          input: { kind: "task", title, projectId: parent.projectId },
        });
        await invoke("links_create", { fromItem: step.id, toItem: parent.id });
        ids.push(step.id);
      }
      return ids;
    }
    case "organize_brief_get":
      return brief(a.projectId as string);
    case "organize_summarize_project": {
      await wait(900);
      const id = a.projectId as string;
      const items = (await allItems()).filter((i) => i.projectId === id);
      const done = items.filter((i) => i.status === "done").length;
      const next = {
        ...brief(id),
        summary: `You finished ${done} of ${items.length} cards. The rest is open and moving; pick the most urgent one next.`,
        summaryAt: new Date().toISOString(),
      };
      briefs.set(id, next);
      return next;
    }
    case "organize_next_action": {
      await wait(700);
      const id = a.projectId as string;
      const open = (await allItems()).find(
        (i) => i.projectId === id && i.status !== "done" && i.kind === "task",
      );
      const next = {
        ...brief(id),
        nextAction: open ? open.title : null,
        nextItemId: open?.id ?? null,
        nextAt: new Date().toISOString(),
      };
      briefs.set(id, next);
      return next;
    }
    case "organize_review":
      return review();
    case "organize_review_summary": {
      await wait(900);
      const r = await review();
      return `You wrapped up ${r.completed.length} things this week. ${r.overdue.length} are overdue, so start there. Then pick one forgotten card and decide whether it still matters.`;
    }
    default:
      return NOT_HANDLED;
  }
}

/** Organize commands, or `NOT_HANDLED` for anything else. */
export function handleOrganize(cmd: string, a: Record<string, unknown>): unknown {
  return cmd.startsWith("organize_") ? organize(cmd, a) : NOT_HANDLED;
}
