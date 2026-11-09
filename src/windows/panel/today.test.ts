import type { Item } from "../../api/types";
import { todayRows } from "./today";

const it_ = (id: string, status: Item["status"], dueAt: string | null) =>
  ({ id, status, dueAt, remindAt: null }) as Item;

test("open items come first, sorted by time, and are counted", () => {
  const { rows, left } = todayRows(
    [
      it_("a", "done", null),
      it_("b", "next", "2026-11-09T15:00:00Z"),
      it_("c", "inbox", "2026-11-09T09:00:00Z"),
    ],
    5,
  );
  expect(rows.map((r) => r.id)).toEqual(["c", "b", "a"]);
  expect(left).toBe(2);
});

test("rows are capped", () => {
  const many = Array.from({ length: 10 }, (_, i) => it_(String(i), "next", null));
  expect(todayRows(many, 7).rows).toHaveLength(7);
  expect(todayRows(many, 7).left).toBe(10);
});
