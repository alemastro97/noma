import { describe, expect, it } from "vitest";
import type { Item } from "../api/types";
import { activeConditions, matchesFilter } from "./filterSpec";

const item = (over: Partial<Item>): Item =>
  ({
    id: "a",
    kind: "task",
    title: "Call Marta",
    body: null,
    status: "inbox",
    dueAt: null,
    remindAt: null,
    projectId: null,
    tags: ["Work"],
    ...over,
  }) as Item;

describe("matchesFilter", () => {
  it("matches everything with an empty spec", () => {
    expect(matchesFilter(item({}), {})).toBe(true);
  });

  it("combines conditions", () => {
    const spec = { text: "marta", types: ["task" as const], tags: ["work"] };
    expect(matchesFilter(item({}), spec)).toBe(true);
    expect(matchesFilter(item({ kind: "idea" }), spec)).toBe(false);
  });

  it("matches cards without a project with 'none'", () => {
    expect(matchesFilter(item({}), { projectIds: ["none"] })).toBe(true);
    expect(matchesFilter(item({ projectId: "p" }), { projectIds: ["none"] })).toBe(false);
  });

  it("filters by local date range and drops undated cards", () => {
    const due = new Date(2026, 10, 11, 9).toISOString();
    expect(matchesFilter(item({ dueAt: due }), { from: "2026-11-11", to: "2026-11-11" })).toBe(
      true,
    );
    expect(matchesFilter(item({ dueAt: due }), { from: "2026-11-12" })).toBe(false);
    expect(matchesFilter(item({}), { to: "2026-11-12" })).toBe(false);
  });

  it("counts conditions", () => {
    expect(activeConditions({ from: "2026-01-01", to: "2026-02-01", types: ["note"] })).toBe(2);
  });
});
