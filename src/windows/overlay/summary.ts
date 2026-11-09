import type { Item, Project } from "../../api/types";
import { kindLabel } from "../../copy";
import { formatDay, formatReminder, toDate } from "../../lib/dates";
import { describeRule } from "../../lib/recurrence";

export interface SummaryChip {
  key: string;
  label: string;
  /** CSS color for the leading dot, if any. */
  dot?: string;
  tone?: "reminder";
}

const KIND_DOT: Record<Item["kind"], string> = {
  task: "var(--noma-task)",
  reminder: "var(--noma-reminder)",
  note: "var(--noma-text-2)",
  idea: "var(--noma-idea)",
  project_ref: "var(--noma-project)",
  event: "var(--noma-project)",
};

/** The chips the overlay shows for a captured card: type, project, date, reminder, repeat. */
export function summarize(item: Item, project: Project | undefined, now: Date): SummaryChip[] {
  const chips: SummaryChip[] = [
    { key: "kind", label: kindLabel[item.kind], dot: KIND_DOT[item.kind] },
  ];
  if (project) chips.push({ key: "project", label: project.name, dot: project.color });
  const due = toDate(item.dueAt);
  if (due) chips.push({ key: "due", label: formatDay(due, now) });
  const remind = toDate(item.remindAt);
  if (remind) chips.push({ key: "remind", label: formatReminder(remind, now), tone: "reminder" });
  if (item.recurrence) chips.push({ key: "repeat", label: describeRule(item.recurrence) });
  return chips;
}

/** Bar heights (0.15..1) for the waveform from recent input levels. */
export function bars(levels: number[], count: number): number[] {
  const out: number[] = [];
  for (let i = 0; i < count; i++) {
    const v = levels[levels.length - count + i] ?? 0;
    out.push(Math.max(0.15, Math.min(1, Math.sqrt(v) * 4)));
  }
  return out;
}
