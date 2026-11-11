/** Repeat presets for the detail panel, and plain-language descriptions of rules. */

import { recurrenceCopy as t } from "../copy";

export type RecurrencePreset = "none" | "daily" | "weekdays" | "weekly" | "monthly" | "yearly";

const PRESETS: RecurrencePreset[] = ["none", "daily", "weekdays", "weekly", "monthly", "yearly"];

/** Presets with their labels in the interface language (read at render time). */
export const RECURRENCE_PRESETS: readonly { value: RecurrencePreset; readonly label: string }[] =
  PRESETS.map((value) => ({
    value,
    get label() {
      return t.presets[value];
    },
  }));

const DAY_CODES = ["SU", "MO", "TU", "WE", "TH", "FR", "SA"] as const;
const WEEKDAYS = "MO,TU,WE,TH,FR";

function parts(rule: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const p of rule.replace(/^RRULE:/, "").split(";")) {
    const [k, v] = p.split("=");
    if (k && v) out[k.toUpperCase()] = v.toUpperCase();
  }
  return out;
}

/** The RRULE for a preset, anchored on the card's date (weekday / day of month). */
export function ruleFor(preset: RecurrencePreset, anchor: Date): string | null {
  switch (preset) {
    case "none":
      return null;
    case "daily":
      return "FREQ=DAILY";
    case "weekdays":
      return `FREQ=WEEKLY;BYDAY=${WEEKDAYS}`;
    case "weekly":
      return `FREQ=WEEKLY;BYDAY=${DAY_CODES[anchor.getDay()]}`;
    case "monthly":
      return `FREQ=MONTHLY;BYMONTHDAY=${anchor.getDate()}`;
    case "yearly":
      return "FREQ=YEARLY";
  }
}

/** Best preset for an existing rule (custom rules map to their frequency). */
export function presetFor(rule: string | null): RecurrencePreset {
  if (!rule) return "none";
  const p = parts(rule);
  switch (p.FREQ) {
    case "DAILY":
      return "daily";
    case "WEEKLY":
      return p.BYDAY === WEEKDAYS ? "weekdays" : "weekly";
    case "MONTHLY":
      return "monthly";
    case "YEARLY":
      return "yearly";
    default:
      return "none";
  }
}

/** "every Monday" · "every weekday" · "every 2 weeks on Monday" · "every month on the 9th". */
export function describeRule(rule: string | null): string {
  if (!rule) return "";
  const p = parts(rule);
  const interval = Number(p.INTERVAL ?? "1");
  const every = (unit: "day" | "week" | "month" | "year") => t.every(unit, interval);
  let text: string;
  switch (p.FREQ) {
    case "DAILY":
      text = every("day");
      break;
    case "WEEKLY": {
      if (p.BYDAY === WEEKDAYS && interval === 1) {
        text = t.everyWeekday;
        break;
      }
      const days = (p.BYDAY ?? "")
        .split(",")
        .filter(Boolean)
        .map((d) => t.days[d.replace(/^[+-]?\d+/, "")] ?? d);
      if (interval === 1 && days.length === 1) text = t.everyDay(days[0]!);
      else text = days.length ? t.onDays(every("week"), days.join(", ")) : every("week");
      break;
    }
    case "MONTHLY":
      text = p.BYMONTHDAY ? t.onMonthDay(every("month"), Number(p.BYMONTHDAY)) : every("month");
      break;
    case "YEARLY":
      text = every("year");
      break;
    default:
      return t.repeats;
  }
  if (p.COUNT) text += t.times(Number(p.COUNT));
  return text;
}
