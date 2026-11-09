import type { Item } from "../../api/types";
import { desktopCopy } from "../../copy";
import { formatTime, toDate } from "../../lib/dates";

/** When a reminder is due, relative to now: "In 15 minutes · 16:00" / "Now · 16:00". */
export function whenLabel(item: Item, now: Date): string {
  const at = toDate(item.snoozedUntil) ?? toDate(item.remindAt);
  if (!at) return "";
  const mins = Math.round((at.getTime() - now.getTime()) / 60_000);
  return `${desktopCopy.inMinutes(mins)} · ${formatTime(at)}`;
}
