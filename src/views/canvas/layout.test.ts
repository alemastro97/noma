import type { Item } from "../../api/types";
import { autoLayout, boardLayout, COLUMN_WIDTH, ORIGIN } from "./layout";

const item = (id: string, kind: Item["kind"] = "task") => ({ id, kind }) as Item;

test("autoLayout fills columns left to right, then the shortest column", () => {
  const pos = autoLayout([item("a", "note"), item("b"), item("c"), item("d"), item("e")]);
  expect(pos.a).toEqual({ x: ORIGIN, y: ORIGIN });
  expect(pos.b).toEqual({ x: ORIGIN + COLUMN_WIDTH, y: ORIGIN });
  expect(pos.d?.x).toBe(ORIGIN + 3 * COLUMN_WIDTH);
  // "a" is a tall note, so the fifth card goes under a short task column.
  expect(pos.e?.x).not.toBe(ORIGIN);
});

test("boardLayout keeps saved positions and slots new cards below them", () => {
  const pos = boardLayout([item("a"), item("b")], { a: { x: 500, y: 300 } });
  expect(pos.a).toEqual({ x: 500, y: 300 });
  expect(pos.b!.y).toBeGreaterThan(300);
});
