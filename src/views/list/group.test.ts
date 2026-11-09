import type { Item } from "../../api/types";
import { groupItems } from "./group";

const now = new Date(2026, 10, 9, 13, 50);
const iso = (d: number, h: number) => new Date(2026, 10, d, h).toISOString();
const item = (id: string, over: Partial<Item> = {}) =>
  ({
    id,
    kind: "task",
    status: "inbox",
    dueAt: null,
    remindAt: null,
    completedAt: null,
    ...over,
  }) as Item;

test("today mode splits by part of day and puts overdue first", () => {
  const groups = groupItems(
    [
      item("late", { dueAt: iso(9, 19) }),
      item("morning", { dueAt: iso(9, 11) }),
      item("overdue", { dueAt: iso(8, 17) }),
      item("afternoon", { remindAt: iso(9, 16) }),
    ],
    "today",
    now,
  );
  expect(groups.map((g) => g.key)).toEqual(["overdue", "morning", "afternoon", "evening"]);
  expect(groups.map((g) => g.label)).toEqual(["Overdue", "Morning", "Afternoon", "Evening"]);
});

test("done items show under Checked only when completed today", () => {
  const groups = groupItems(
    [
      item("today", { status: "done", completedAt: iso(9, 10) }),
      item("old", { status: "done", completedAt: iso(2, 10) }),
    ],
    "today",
    now,
  );
  expect(groups).toEqual([
    { key: "checked", label: "Checked", items: [expect.objectContaining({ id: "today" })] },
  ]);
});

test("timeline mode buckets by day distance", () => {
  const groups = groupItems(
    [
      item("t", { dueAt: iso(9, 18) }),
      item("tm", { dueAt: iso(10, 9) }),
      item("wk", { dueAt: iso(13, 9) }),
      item("later", { dueAt: iso(30, 9) }),
      item("none"),
    ],
    "timeline",
    now,
  );
  expect(groups.map((g) => g.key)).toEqual(["today", "tomorrow", "thisWeek", "later", "noDate"]);
});
