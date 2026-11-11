import type { Item } from "../../api/types";
import { anchorDate, startOfDay, toDate } from "../../lib/dates";

/** Timeline placement: one column per day, cards stacked in lanes; events span to `endsAt`. */

export interface Bar {
  item: Item;
  /** First and last visible day column (0-based, inclusive). */
  from: number;
  to: number;
  lane: number;
  /** The card starts before / ends after the visible range. */
  clippedStart: boolean;
  clippedEnd: boolean;
}

const DAY = 86_400_000;

function dayIndex(d: Date, start: Date): number {
  return Math.round((startOfDay(d).getTime() - start.getTime()) / DAY);
}

/** Last day an item covers: its end for events (an end at midnight belongs to the day before). */
function lastDay(item: Item, at: Date): Date {
  const end = item.kind === "event" ? toDate(item.endsAt) : null;
  if (!end || end <= at) return at;
  const midnight = end.getHours() === 0 && end.getMinutes() === 0;
  return midnight ? new Date(end.getTime() - 1) : end;
}

export function layoutBars(items: Item[], start: Date, days: number): Bar[] {
  const first = startOfDay(start);
  const spans = items
    .filter((i) => i.kind !== "project_ref")
    .flatMap((item) => {
      const at = anchorDate(item);
      if (!at) return [];
      const from = dayIndex(at, first);
      const to = dayIndex(lastDay(item, at), first);
      if (to < 0 || from >= days) return [];
      return [{ item, from, to, at: at.getTime() }];
    })
    .sort((a, b) => a.from - b.from || b.to - b.from - (a.to - a.from) || a.at - b.at);

  const laneEnds: number[] = [];
  return spans.map(({ item, from, to }) => {
    const visibleFrom = Math.max(0, from);
    const visibleTo = Math.min(days - 1, to);
    let lane = laneEnds.findIndex((end) => end < visibleFrom);
    if (lane < 0) lane = laneEnds.length;
    laneEnds[lane] = visibleTo;
    return {
      item,
      from: visibleFrom,
      to: visibleTo,
      lane,
      clippedStart: from < 0,
      clippedEnd: to >= days,
    };
  });
}
