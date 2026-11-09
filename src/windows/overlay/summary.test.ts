import type { Item } from "../../api/types";
import { bars, summarize } from "./summary";

const now = new Date(2026, 10, 9, 10);
const item = (over: Partial<Item>) =>
  ({
    kind: "task",
    dueAt: null,
    remindAt: null,
    recurrence: null,
    ...over,
  }) as Item;

test("summarize lists type, project, date, reminder and repeat", () => {
  const chips = summarize(
    item({
      kind: "reminder",
      remindAt: new Date(2026, 10, 13, 9).toISOString(),
      recurrence: "FREQ=WEEKLY;BYDAY=FR",
    }),
    { id: "p", name: "Spring launch", color: "#79B8F5", boardId: "b", createdAt: "" },
    now,
  );
  expect(chips.map((c) => c.label)).toEqual([
    "Reminder",
    "Spring launch",
    "Fri 13, 09:00",
    "every Friday",
  ]);
});

test("bars are clamped and padded on the left", () => {
  const b = bars([0, 1], 4);
  expect(b).toHaveLength(4);
  expect(b[0]).toBe(0.15);
  expect(b[3]).toBe(1);
});
