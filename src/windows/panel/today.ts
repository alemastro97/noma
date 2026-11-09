import type { Item } from "../../api/types";
import { anchorDate } from "../../lib/dates";

/** Open items first (by time), then done ones; `left` counts the open ones. */
export function todayRows(items: Item[], max: number): { rows: Item[]; left: number } {
  const time = (i: Item) => anchorDate(i)?.getTime() ?? Number.MAX_SAFE_INTEGER;
  const open = items.filter((i) => i.status !== "done").sort((a, b) => time(a) - time(b));
  const done = items.filter((i) => i.status === "done");
  return { rows: [...open, ...done].slice(0, max), left: open.length };
}
