import type { Item } from "../../api/types";
import { groupByStatus } from "./group";

const now = new Date(2026, 10, 9, 12);
const item = (id: string, over: Partial<Item>) =>
  ({ id, status: "inbox", completedAt: null, ...over }) as Item;

test("groups by status and keeps only recent done items, newest first", () => {
  const cols = groupByStatus(
    [
      item("a", {}),
      item("b", { status: "doing" }),
      item("old", { status: "done", completedAt: new Date(2026, 9, 1).toISOString() }),
      item("d1", { status: "done", completedAt: new Date(2026, 10, 8).toISOString() }),
      item("d2", { status: "done", completedAt: new Date(2026, 10, 9).toISOString() }),
    ],
    now,
  );
  expect(cols.inbox.map((i) => i.id)).toEqual(["a"]);
  expect(cols.next).toEqual([]);
  expect(cols.doing.map((i) => i.id)).toEqual(["b"]);
  expect(cols.done.map((i) => i.id)).toEqual(["d2", "d1"]);
});
