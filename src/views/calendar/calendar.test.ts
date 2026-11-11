import type { Item } from "../../api/types";
import { dayKey, itemsByDay, reschedulePatch, step, visibleDays } from "./calendar";

const base: Item = {
  id: "a",
  kind: "task",
  title: "Call",
  body: null,
  status: "inbox",
  priority: "normal",
  dueAt: null,
  endsAt: null,
  remindAt: null,
  snoozedUntil: null,
  recurrence: null,
  projectId: null,
  createdAt: "2026-11-01T08:00:00Z",
  updatedAt: "2026-11-01T08:00:00Z",
  completedAt: null,
  tags: [],
  linkCount: 0,
  hint: null,
  capturing: false,
};

test("month grid covers whole Monday-first weeks", () => {
  const days = visibleDays(new Date(2026, 10, 11), "month");
  expect(days.length % 7).toBe(0);
  expect(days[0]!.getDay()).toBe(1);
  expect(dayKey(days[0]!)).toBe("2026-10-26");
  expect(dayKey(days[days.length - 1]!)).toBe("2026-12-06");
});

test("week view starts on Monday and steps by 7 days", () => {
  const days = visibleDays(new Date(2026, 10, 11), "week");
  expect(days.map(dayKey)[0]).toBe("2026-11-09");
  expect(dayKey(step(new Date(2026, 10, 11), "week", 1))).toBe("2026-11-18");
});

test("cards are grouped by their due day", () => {
  const due = new Date(2026, 10, 12, 9).toISOString();
  const map = itemsByDay([
    { ...base, dueAt: due },
    { ...base, id: "b" },
  ]);
  expect(map.get("2026-11-12")?.map((i) => i.id)).toEqual(["a"]);
  expect(map.size).toBe(1);
});

test("rescheduling keeps the time of day and shifts the reminder too", () => {
  const item = {
    ...base,
    dueAt: new Date(2026, 10, 12, 16).toISOString(),
    remindAt: new Date(2026, 10, 12, 15).toISOString(),
  };
  const patch = reschedulePatch(item, new Date(2026, 10, 14))!;
  expect(new Date(patch.dueAt!).getDate()).toBe(14);
  expect(new Date(patch.dueAt!).getHours()).toBe(16);
  expect(new Date(patch.remindAt!).getHours()).toBe(15);
  expect(reschedulePatch(item, new Date(2026, 10, 12))).toBeNull();
});
