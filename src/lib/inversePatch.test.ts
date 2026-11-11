import { describe, expect, it } from "vitest";
import type { Item } from "../api/types";
import { inversePatch } from "./inversePatch";

const item = {
  id: "a",
  title: "Old",
  dueAt: null,
  tags: ["x"],
  status: "next",
} as unknown as Item;

describe("inversePatch", () => {
  it("restores only the keys the patch touches", () => {
    expect(inversePatch(item, { title: "New", dueAt: "2026-11-11T09:00:00Z" })).toEqual({
      title: "Old",
      dueAt: null,
    });
  });

  it("copies arrays", () => {
    const inv = inversePatch(item, { tags: [] });
    expect(inv.tags).toEqual(["x"]);
    expect(inv.tags).not.toBe(item.tags);
  });
});
