/**
 * Date helpers for card metadata. All output is English (en-GB: 24 h clock,
 * "9 November"). `now` is always passed in so the functions are pure and testable.
 */

const LOCALE = "en-GB";
const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** Parses a wire timestamp. Truncates sub-millisecond digits WebKit may reject. */
export function parseIso(iso: string): Date {
  return new Date(iso.replace(/(\.\d{3})\d+/, "$1").replace(" ", "T"));
}

export function toDate(value: string | Date | null | undefined): Date | null {
  if (!value) return null;
  const d = typeof value === "string" ? parseIso(value) : value;
  return Number.isNaN(d.getTime()) ? null : d;
}

export function startOfDay(d: Date): Date {
  const s = new Date(d);
  s.setHours(0, 0, 0, 0);
  return s;
}

/** Whole calendar days from `now` to `d` in local time (negative = past). */
export function dayDiff(d: Date, now: Date): number {
  return Math.round((startOfDay(d).getTime() - startOfDay(now).getTime()) / DAY);
}

export function formatTime(d: Date): string {
  return d.toLocaleTimeString(LOCALE, { hour: "2-digit", minute: "2-digit", hour12: false });
}

function hasTime(d: Date): boolean {
  return d.getHours() !== 0 || d.getMinutes() !== 0;
}

/** "Today" · "Tomorrow" · "Yesterday" · "Thu 12" (within a week) · "Apr 2" · "Apr 2, 2027". */
export function formatDay(d: Date, now: Date): string {
  const diff = dayDiff(d, now);
  if (diff === 0) return "Today";
  if (diff === 1) return "Tomorrow";
  if (diff === -1) return "Yesterday";
  if (diff > 1 && diff < 7) {
    const weekday = d.toLocaleDateString(LOCALE, { weekday: "short" });
    return `${weekday} ${d.getDate()}`;
  }
  const month = d.toLocaleDateString("en-US", { month: "short" });
  return d.getFullYear() === now.getFullYear()
    ? `${month} ${d.getDate()}`
    : `${month} ${d.getDate()}, ${d.getFullYear()}`;
}

/** Task deadline: "Thu 12 · 18:00", "Today · 11:00", or just the day when no time. */
export function formatDue(d: Date, now: Date): string {
  return hasTime(d) ? `${formatDay(d, now)} · ${formatTime(d)}` : formatDay(d, now);
}

/** Reminder headline: "Today, 16:00". */
export function formatReminder(d: Date, now: Date): string {
  return `${formatDay(d, now)}, ${formatTime(d)}`;
}

/** "in 2h 10m" · "in 5m" · "in 3d" · "2h ago" · "now". */
export function relativeFromNow(d: Date, now: Date): string {
  const delta = d.getTime() - now.getTime();
  const abs = Math.abs(delta);
  if (abs < MINUTE) return "now";
  let text: string;
  if (abs < HOUR) text = `${Math.round(abs / MINUTE)}m`;
  else if (abs < DAY) {
    const h = Math.floor(abs / HOUR);
    const m = Math.round((abs - h * HOUR) / MINUTE);
    text = m ? `${h}h ${m}m` : `${h}h`;
  } else text = `${Math.round(abs / DAY)}d`;
  return delta > 0 ? `in ${text}` : `${text} ago`;
}

/** "edited just now" · "edited 5m ago" · "edited yesterday" · "edited Thu 12". */
export function editedAgo(d: Date, now: Date): string {
  const abs = now.getTime() - d.getTime();
  if (abs < MINUTE) return "edited just now";
  if (abs < HOUR) return `edited ${Math.round(abs / MINUTE)}m ago`;
  const diff = dayDiff(d, now);
  if (diff === 0) return `edited ${Math.floor(abs / HOUR)}h ago`;
  if (diff === -1) return "edited yesterday";
  return `edited ${formatDay(d, now)}`;
}

/** Event date block: { weekday: "WED", day: "11" }. */
export function eventBlock(d: Date): { weekday: string; day: string } {
  return {
    weekday: d.toLocaleDateString(LOCALE, { weekday: "short" }).toUpperCase(),
    day: String(d.getDate()),
  };
}

/** "14:30 – 15:30", or a single time when there is no end. */
export function timeRange(start: Date, end: Date | null): string {
  return end ? `${formatTime(start)} – ${formatTime(end)}` : formatTime(start);
}

/** "Monday, 9 November". */
export function longDate(d: Date): string {
  return d.toLocaleDateString(LOCALE, { weekday: "long", day: "numeric", month: "long" });
}

export function isOverdue(d: Date, now: Date): boolean {
  return d.getTime() < now.getTime();
}

export type PartOfDay = "morning" | "afternoon" | "evening";

export function partOfDay(d: Date): PartOfDay {
  const h = d.getHours();
  if (h < 12) return "morning";
  if (h < 18) return "afternoon";
  return "evening";
}

/** `YYYY-MM-DDTHH:mm` in local time, for `<input type="datetime-local">`. */
export function toLocalInput(iso: string | null): string {
  const d = toDate(iso);
  if (!d) return "";
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** Inverse of `toLocalInput`: local wall time → UTC ISO string (or null when empty). */
export function fromLocalInput(value: string): string | null {
  if (!value) return null;
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? null : d.toISOString();
}

/** The date a card is "about": due, else reminder. */
export function anchorDate(item: { dueAt: string | null; remindAt: string | null }): Date | null {
  return toDate(item.dueAt) ?? toDate(item.remindAt);
}
