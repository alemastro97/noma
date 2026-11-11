import { describe, expect, it } from "vitest";
import { rangeBetween } from "./selection";

describe("rangeBetween", () => {
  const order = ["a", "b", "c", "d"];
  it("spans either direction inclusively", () => {
    expect(rangeBetween(order, "b", "d")).toEqual(["b", "c", "d"]);
    expect(rangeBetween(order, "d", "b")).toEqual(["b", "c", "d"]);
  });
  it("falls back to the clicked card", () => {
    expect(rangeBetween(order, "x", "c")).toEqual(["c"]);
  });
});
