import { describe, expect, it } from "vitest";
import { seedPosition, tick, type SimNode } from "./force";

function nodes(n: number): SimNode[] {
  return Array.from({ length: n }, (_, i) => ({ id: String(i), ...seedPosition(i), vx: 0, vy: 0 }));
}

describe("force layout", () => {
  it("pulls linked nodes closer than unlinked ones", () => {
    const ns = nodes(3);
    for (let k = 0; k < 400; k++) tick(ns, [[0, 1]], 1 - k / 400);
    const d = (a: SimNode, b: SimNode) => Math.hypot(a.x - b.x, a.y - b.y);
    expect(d(ns[0]!, ns[1]!)).toBeLessThan(d(ns[0]!, ns[2]!));
  });

  it("keeps fixed nodes in place", () => {
    const ns = nodes(2);
    ns[0]!.fixed = true;
    const { x, y } = ns[0]!;
    tick(ns, [[0, 1]], 1);
    expect(ns[0]!.x).toBe(x);
    expect(ns[0]!.y).toBe(y);
  });
});
