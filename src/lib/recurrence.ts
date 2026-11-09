/** Repeat presets for the detail panel, and plain-English descriptions of rules. */

export type RecurrencePreset = "none" | "daily" | "weekdays" | "weekly" | "monthly" | "yearly";

export const RECURRENCE_PRESETS: { value: RecurrencePreset; label: string }[] = [
  { value: "none", label: "Doesn't repeat" },
  { value: "daily", label: "Every day" },
  { value: "weekdays", label: "Every weekday" },
  { value: "weekly", label: "Every week" },
  { value: "monthly", label: "Every month" },
  { value: "yearly", label: "Every year" },
];

const DAY_CODES = ["SU", "MO", "TU", "WE", "TH", "FR", "SA"] as const;
const DAY_NAMES: Record<string, string> = {
  MO: "Monday",
  TU: "Tuesday",
  WE: "Wednesday",
  TH: "Thursday",
  FR: "Friday",
  SA: "Saturday",
  SU: "Sunday",
};
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

function ordinal(n: number): string {
  const s = ["th", "st", "nd", "rd"];
  const v = n % 100;
  return n + (s[(v - 20) % 10] ?? s[v] ?? s[0]!);
}

/** "every Monday" · "every weekday" · "every 2 weeks on Monday" · "every month on the 9th". */
export function describeRule(rule: string | null): string {
  if (!rule) return "";
  const p = parts(rule);
  const interval = Number(p.INTERVAL ?? "1");
  const every = (unit: string) => (interval > 1 ? `every ${interval} ${unit}s` : `every ${unit}`);
  let text: string;
  switch (p.FREQ) {
    case "DAILY":
      text = every("day");
      break;
    case "WEEKLY": {
      if (p.BYDAY === WEEKDAYS && interval === 1) {
        text = "every weekday";
        break;
      }
      const days = (p.BYDAY ?? "")
        .split(",")
        .filter(Boolean)
        .map((d) => DAY_NAMES[d.replace(/^[+-]?\d+/, "")] ?? d);
      if (interval === 1 && days.length === 1) text = `every ${days[0]}`;
      else text = days.length ? `${every("week")} on ${days.join(", ")}` : every("week");
      break;
    }
    case "MONTHLY":
      text = p.BYMONTHDAY
        ? `${every("month")} on the ${ordinal(Number(p.BYMONTHDAY))}`
        : every("month");
      break;
    case "YEARLY":
      text = every("year");
      break;
    default:
      return "repeats";
  }
  if (p.COUNT) text += `, ${p.COUNT} times`;
  return text;
}
