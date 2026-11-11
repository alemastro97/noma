import type { Item } from "../api/types";
import type { FilterSpec } from "../api/workspace";
import { anchorDate } from "./dates";

function localDay(d: Date): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

/** Number of conditions set (for the filter button badge). */
export function activeConditions(spec: FilterSpec): number {
  let n = 0;
  if (spec.text?.trim()) n++;
  if (spec.types?.length) n++;
  if (spec.tags?.length) n++;
  if (spec.projectIds?.length) n++;
  if (spec.statuses?.length) n++;
  if (spec.from || spec.to) n++;
  return n;
}

export function isEmptySpec(spec: FilterSpec): boolean {
  return activeConditions(spec) === 0;
}

/** True when `item` satisfies every condition set in `spec`. */
export function matchesFilter(item: Item, spec: FilterSpec): boolean {
  const text = spec.text?.trim().toLowerCase();
  if (text && !`${item.title} ${item.body ?? ""}`.toLowerCase().includes(text)) return false;
  if (spec.types?.length && !spec.types.includes(item.kind)) return false;
  if (spec.statuses?.length && !spec.statuses.includes(item.status)) return false;
  if (spec.tags?.length) {
    const tags = item.tags.map((t) => t.toLowerCase());
    if (!spec.tags.some((t) => tags.includes(t.toLowerCase()))) return false;
  }
  if (spec.projectIds?.length) {
    const key = item.projectId ?? "none";
    if (!spec.projectIds.includes(key)) return false;
  }
  if (spec.from || spec.to) {
    const anchor = anchorDate(item);
    if (!anchor) return false;
    const day = localDay(anchor);
    if (spec.from && day < spec.from) return false;
    if (spec.to && day > spec.to) return false;
  }
  return true;
}
