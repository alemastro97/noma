import { invoke } from "@tauri-apps/api/core";
import type { Item } from "../api/types";
import { EMPTY_COUNTS } from "../api/types";
import { useItemsStore } from "./items";

const item = (over: Partial<Item>): Item => ({
  id: "i1",
  kind: "task",
  title: "Hello",
  body: null,
  status: "inbox",
  priority: "normal",
  dueAt: null,
  remindAt: null,
  snoozedUntil: null,
  recurrence: null,
  projectId: null,
  createdAt: "2026-11-09T10:00:00Z",
  updatedAt: "2026-11-09T10:00:00Z",
  completedAt: null,
  ...over,
});

const mocked = vi.mocked(invoke);

beforeEach(() => {
  mocked.mockReset();
  useItemsStore.setState({
    items: {},
    order: [],
    counts: EMPTY_COUNTS,
    loading: false,
    error: null,
  });
});

test("load('inbox') stores items by id in order and clears loading", async () => {
  mocked.mockResolvedValueOnce([item({ id: "a" }), item({ id: "b" })]);
  const p = useItemsStore.getState().load({ kind: "inbox" });
  expect(useItemsStore.getState().loading).toBe(true);
  await p;
  const s = useItemsStore.getState();
  expect(s.loading).toBe(false);
  expect(s.order).toEqual(["a", "b"]);
  expect(s.items.a?.id).toBe("a");
  expect(mocked).toHaveBeenCalledWith("items_list", { filter: { kind: "inbox" } });
});

test("create calls items_create and inserts the result at the top", async () => {
  mocked.mockResolvedValueOnce(item({ id: "new", title: "Created" }));
  useItemsStore.setState({ order: ["old"], items: { old: item({ id: "old" }) } });
  const created = await useItemsStore.getState().create({ kind: "task", title: "Created" });
  expect(created.id).toBe("new");
  expect(mocked).toHaveBeenCalledWith("items_create", {
    input: { kind: "task", title: "Created" },
  });
  expect(useItemsStore.getState().order).toEqual(["new", "old"]);
});

test("complete is optimistic then reconciled with the server item", async () => {
  useItemsStore.setState({ order: ["a"], items: { a: item({ id: "a" }) } });
  let resolve!: (v: Item) => void;
  mocked.mockReturnValueOnce(new Promise<Item>((r) => (resolve = r)));
  const p = useItemsStore.getState().complete("a");
  expect(useItemsStore.getState().items.a?.status).toBe("done");
  resolve(item({ id: "a", status: "done", completedAt: "2026-11-09T11:00:00Z" }));
  await p;
  expect(useItemsStore.getState().items.a?.completedAt).toBe("2026-11-09T11:00:00Z");
});

test("complete rolls back when invoke rejects", async () => {
  useItemsStore.setState({ order: ["a"], items: { a: item({ id: "a" }) } });
  mocked.mockRejectedValueOnce(new Error("boom"));
  await useItemsStore.getState().complete("a");
  const s = useItemsStore.getState();
  expect(s.items.a?.status).toBe("inbox");
  expect(s.error).toBe("boom");
});

test("remove deletes locally and on the server", async () => {
  useItemsStore.setState({
    order: ["a", "b"],
    items: { a: item({ id: "a" }), b: item({ id: "b" }) },
  });
  mocked.mockResolvedValueOnce(undefined);
  await useItemsStore.getState().remove("a");
  expect(useItemsStore.getState().order).toEqual(["b"]);
  expect(mocked).toHaveBeenCalledWith("items_delete", { id: "a" });
});

test("refreshCounts updates counts", async () => {
  mocked.mockResolvedValueOnce({ ...EMPTY_COUNTS, inbox: 3, byProject: { p1: 2 } });
  await useItemsStore.getState().refreshCounts();
  expect(useItemsStore.getState().counts.inbox).toBe(3);
  expect(useItemsStore.getState().counts.byProject.p1).toBe(2);
});
