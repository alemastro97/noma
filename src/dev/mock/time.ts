/** Browser mock for the time commands (focus timer, habits, reminder offsets, snooze). */
import { invoke } from "@tauri-apps/api/core";
import type { FocusState, Habit, ItemTime } from "../../api/time";
import type { Item } from "../../api/types";

export const NOT_HANDLED = Symbol("not handled");

const offsets = new Map<string, number[]>();
const habits = new Set<string>();
const focusLog = new Map<string, { sessions: number; minutes: number }>();
let focus: FocusState | null = null;

function stats(id: string) {
  return focusLog.get(id) ?? { sessions: 0, minutes: 0 };
}

function endFocus(completed: boolean) {
  if (!focus) return;
  const s = stats(focus.itemId);
  const minutes = Math.round((Date.now() - new Date(focus.startedAt).getTime()) / 60_000);
  focusLog.set(focus.itemId, {
    sessions: s.sessions + (completed ? 1 : 0),
    minutes: s.minutes + minutes,
  });
  focus = null;
}

/** A plausible streak for demo purposes. */
const demoStats = { streak: 4, last7: [true, false, true, true, true, true, false] };

export function handleTime(cmd: string, a: Record<string, unknown>): unknown {
  switch (cmd) {
    case "reminder_snooze_until":
    case "recap_open":
      return null;
    case "time_item_info": {
      const id = String(a.itemId);
      const info: ItemTime = {
        offsets: offsets.get(id) ?? [],
        habit: habits.has(id),
        habitStats: habits.has(id) ? demoStats : null,
        focus: stats(id),
      };
      return info;
    }
    case "reminder_offsets_set": {
      const list = [...new Set(a.minutes as number[])].sort((x, y) => x - y);
      offsets.set(String(a.itemId), list);
      return list;
    }
    case "habit_set":
      if (a.on) habits.add(String(a.itemId));
      else habits.delete(String(a.itemId));
      return null;
    case "habits_list":
      return Promise.all([...habits].map((id) => invoke<Item | null>("items_get", { id }))).then(
        (items) =>
          items
            .filter((i): i is Item => !!i && i.status !== "done" && !!i.recurrence)
            .map((item): Habit => ({ item, ...demoStats })),
      );
    case "focus_current":
      return focus;
    case "focus_start": {
      endFocus(false);
      const minutes = Number(a.minutes ?? 25);
      const now = Date.now();
      return invoke<Item | null>("items_get", { id: a.itemId }).then((item) => {
        focus = {
          itemId: String(a.itemId),
          title: item?.title ?? "",
          startedAt: new Date(now).toISOString(),
          endsAt: new Date(now + minutes * 60_000).toISOString(),
          minutes,
        };
        return focus;
      });
    }
    case "focus_stop":
      endFocus(false);
      return null;
    default:
      return NOT_HANDLED;
  }
}
