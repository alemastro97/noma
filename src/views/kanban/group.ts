import { STATUSES, type Item, type Status } from "../../api/types";
import { toDate } from "../../lib/dates";

const DONE_WINDOW_DAYS = 7;

/** Splits items into Kanban columns. Done shows only the last 7 days, newest first. */
export function groupByStatus(items: Item[], now: Date): Record<Status, Item[]> {
  const columns = Object.fromEntries(STATUSES.map((s) => [s, [] as Item[]])) as Record<
    Status,
    Item[]
  >;
  const cutoff = now.getTime() - DONE_WINDOW_DAYS * 24 * 60 * 60 * 1000;
  for (const item of items) {
    if (item.status === "done") {
      const completed = toDate(item.completedAt);
      if (!completed || completed.getTime() < cutoff) continue;
    }
    columns[item.status].push(item);
  }
  columns.done.sort(
    (a, b) => (toDate(b.completedAt)?.getTime() ?? 0) - (toDate(a.completedAt)?.getTime() ?? 0),
  );
  return columns;
}
