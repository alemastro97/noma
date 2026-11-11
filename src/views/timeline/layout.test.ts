import type { Item } from "../../api/types";
import { layoutBars } from "./layout";

const item = (id: string, over: Partial<Item>): Item => ({
  id,
  kind: "task",
  title: id,
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
  ...over,
});

const day = (d: number, h = 10) => new Date(2026, 10, d, h).toISOString();

test("events span to their end and push overlapping cards to another lane", () => {
  const bars = layoutBars(
    [
      item("trip", { kind: "event", dueAt: day(10), endsAt: day(12) }),
      item("call", { dueAt: day(11) }),
      item("later", { dueAt: day(13) }),
    ],
    new Date(2026, 10, 9),
    7,
  );
  const by = Object.fromEntries(bars.map((b) => [b.item.id, b]));
  expect(by.trip).toMatchObject({ from: 1, to: 3, lane: 0 });
  expect(by.call).toMatchObject({ from: 2, to: 2, lane: 1 });
  expect(by.later).toMatchObject({ from: 4, lane: 0 });
});

test("cards outside the range are dropped and long spans are clipped", () => {
  const bars = layoutBars(
    [
      item("old", { dueAt: day(1) }),
      item("long", { kind: "event", dueAt: day(5), endsAt: day(30) }),
    ],
    new Date(2026, 10, 9),
    7,
  );
  expect(bars.map((b) => b.item.id)).toEqual(["long"]);
  expect(bars[0]).toMatchObject({ from: 0, to: 6, clippedStart: true, clippedEnd: true });
});
