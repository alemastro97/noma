import type { Item, ItemPatch } from "../../api/types";
import { anchorDate, startOfDay, toDate } from "../../lib/dates";

/** Calendar math in local time; weeks start on Monday. Pure: dates are passed in. */

export type CalendarMode = "month" | "week";

/** Deadline hour given to a card quick-added on a calendar day (same as Today). */
export const QUICK_ADD_HOUR = 18;

export function addDays(d: Date, n: number): Date {
  const r = new Date(d);
  r.setDate(r.getDate() + n);
  return r;
}

export function startOfWeek(d: Date): Date {
  const s = startOfDay(d);
  return addDays(s, -((s.getDay() + 6) % 7));
}

/** "2026-11-09" in local time. */
export function dayKey(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** The days shown: whole weeks covering the anchor's month, or the anchor's week. */
export function visibleDays(anchor: Date, mode: CalendarMode): Date[] {
  if (mode === "week") {
    const start = startOfWeek(anchor);
    return Array.from({ length: 7 }, (_, i) => addDays(start, i));
  }
  const first = new Date(anchor.getFullYear(), anchor.getMonth(), 1);
  const last = new Date(anchor.getFullYear(), anchor.getMonth() + 1, 0);
  const start = startOfWeek(first);
  const end = addDays(startOfWeek(last), 6);
  const days: Date[] = [];
  for (let d = start; d <= end; d = addDays(d, 1)) days.push(d);
  return days;
}

/** Moves the anchor a month or a week. */
export function step(anchor: Date, mode: CalendarMode, dir: 1 | -1): Date {
  if (mode === "week") return addDays(anchor, 7 * dir);
  return new Date(anchor.getFullYear(), anchor.getMonth() + dir, 1);
}

/** Dated cards grouped by local day of their due (else reminder) date, earliest first. */
export function itemsByDay(items: Item[]): Map<string, Item[]> {
  const map = new Map<string, Item[]>();
  const dated = items
    .map((item) => ({ item, at: anchorDate(item) }))
    .filter((x): x is { item: Item; at: Date } => x.at !== null && x.item.kind !== "project_ref")
    .sort((a, b) => a.at.getTime() - b.at.getTime());
  for (const { item, at } of dated) {
    const key = dayKey(at);
    const list = map.get(key);
    if (list) list.push(item);
    else map.set(key, [item]);
  }
  return map;
}

function shiftDays(iso: string | null, days: number): string | null {
  const d = toDate(iso);
  return d ? addDays(d, days).toISOString() : null;
}

/** Patch that moves a card to `day`, keeping its times of day and the gaps between its dates. */
export function reschedulePatch(item: Item, day: Date): ItemPatch | null {
  const at = anchorDate(item);
  if (!at) return null;
  const days = Math.round((startOfDay(day).getTime() - startOfDay(at).getTime()) / 86_400_000);
  if (days === 0) return null;
  const patch: ItemPatch = {};
  if (item.dueAt) patch.dueAt = shiftDays(item.dueAt, days);
  if (item.endsAt) patch.endsAt = shiftDays(item.endsAt, days);
  if (item.remindAt) patch.remindAt = shiftDays(item.remindAt, days);
  return patch;
}

/** Due date for a card quick-added on `day`. */
export function quickAddDue(day: Date): string {
  const d = startOfDay(day);
  d.setHours(QUICK_ADD_HOUR, 0, 0, 0);
  return d.toISOString();
}
