import type { Item } from "../../api/types";
import { listCopy } from "../../copy";
import { anchorDate, dayDiff, partOfDay, toDate } from "../../lib/dates";

export interface ListGroup {
  key: string;
  label: string;
  items: Item[];
}

export type GroupMode = "today" | "timeline";

function byAnchor(a: Item, b: Item): number {
  const da = anchorDate(a)?.getTime() ?? Infinity;
  const db = anchorDate(b)?.getTime() ?? Infinity;
  return da - db;
}

/**
 * Groups items for the List view.
 * - `today`: Overdue · Morning · Afternoon · Evening · Checked (done today).
 * - `timeline`: Overdue · Today · Tomorrow · This week · Later · No date · Checked.
 * Done items appear only under Checked, and only when completed today.
 */
export function groupItems(items: Item[], mode: GroupMode, now: Date): ListGroup[] {
  const buckets = new Map<string, Item[]>();
  const push = (key: string, item: Item) => {
    const list = buckets.get(key) ?? [];
    list.push(item);
    buckets.set(key, list);
  };

  for (const item of items) {
    if (item.status === "done") {
      const completed = toDate(item.completedAt);
      if (completed && dayDiff(completed, now) === 0) push("checked", item);
      continue;
    }
    const anchor = anchorDate(item);
    if (!anchor) {
      push(mode === "today" ? "morning" : "noDate", item);
      continue;
    }
    const diff = dayDiff(anchor, now);
    if (diff < 0) {
      push("overdue", item);
      continue;
    }
    if (mode === "today") push(diff === 0 ? partOfDay(anchor) : "evening", item);
    else if (diff === 0) push("today", item);
    else if (diff === 1) push("tomorrow", item);
    else if (diff < 7) push("thisWeek", item);
    else push("later", item);
  }

  const order =
    mode === "today"
      ? (["overdue", "morning", "afternoon", "evening", "checked"] as const)
      : (["overdue", "today", "tomorrow", "thisWeek", "later", "noDate", "checked"] as const);

  return order
    .filter((key) => buckets.has(key))
    .map((key) => ({
      key,
      label: listCopy[key],
      items:
        key === "noDate" || key === "checked"
          ? buckets.get(key)!
          : buckets.get(key)!.sort(byAnchor),
    }));
}
