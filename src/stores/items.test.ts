import { invoke } from "@tauri-apps/api/core";
import type { Item } from "../api/types";
import { EMPTY_COUNTS } from "../api/types";
import { useItemsStore } from "./items";
import { useToastsStore } from "./toasts";

const item = (over: Partial<Item>): Item => ({
  id: "i1",
  kind: "task",
  title: "Hello",
  body: null,
  status: "inbox",
  priority: "normal",
  dueAt: null,
  endsAt: null,
  remindAt: null,
  snoozedUntil: null,
  recurrence: null,
  projectId: null,
  createdAt: "2026-11-09T10:00:00Z",
  updatedAt: "2026-11-09T10:00:00Z",
  completedAt: null,
  tags: [],
  linkCount: 0,
  hint: null,
  capturing: false,
  ...over,
});

const mocked = vi.mocked(invoke);

beforeEach(() => {
  mocked.mockReset();
  useToastsStore.setState({ toasts: [] });
  useItemsStore.setState({
    items: {},
    order: [],
    counts: EMPTY_COUNTS,
    loading: false,
    error: null,
    lastFilter: null,
    lastOptions: {},
  });
});

test("load stores items by id in order and passes includeDone", async () => {
  mocked.mockResolvedValueOnce([item({ id: "a" }), item({ id: "b" })]);
  const p = useItemsStore.getState().load({ kind: "inbox" }, { includeDone: true });
  expect(useItemsStore.getState().loading).toBe(true);
  await p;
  const s = useItemsStore.getState();
  expect(s.loading).toBe(false);
  expect(s.order).toEqual(["a", "b"]);
  expect(mocked).toHaveBeenCalledWith("items_list", {
    filter: { kind: "inbox" },
    includeDone: true,
  });
});

test("a slower earlier load never overwrites a newer one", async () => {
  let resolveSlow!: (v: Item[]) => void;
  mocked
    .mockReturnValueOnce(new Promise<Item[]>((r) => (resolveSlow = r)))
    .mockResolvedValueOnce([item({ id: "inbox" })]);
  const slow = useItemsStore.getState().load({ kind: "upcoming" });
  await useItemsStore.getState().load({ kind: "inbox" });
  resolveSlow([item({ id: "upcoming" })]);
  await slow;
  expect(useItemsStore.getState().order).toEqual(["inbox"]);
});

test("create inserts the result at the top and marks it fresh", async () => {
  mocked.mockResolvedValueOnce(item({ id: "new", title: "Created" }));
  useItemsStore.setState({ order: ["old"], items: { old: item({ id: "old" }) } });
  const created = await useItemsStore.getState().create({ kind: "task", title: "Created" });
  expect(created?.id).toBe("new");
  expect(useItemsStore.getState().order).toEqual(["new", "old"]);
  expect(useItemsStore.getState().freshId).toBe("new");
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

test("complete on a recurring item is not optimistically done", async () => {
  useItemsStore.setState({
    order: ["r"],
    items: { r: item({ id: "r", recurrence: "FREQ=DAILY", dueAt: "2026-11-09T09:00:00Z" }) },
  });
  let resolve!: (v: Item) => void;
  mocked.mockReturnValueOnce(new Promise<Item>((r) => (resolve = r)));
  const p = useItemsStore.getState().complete("r");
  expect(useItemsStore.getState().items.r?.status).toBe("inbox");
  resolve(item({ id: "r", recurrence: "FREQ=DAILY", dueAt: "2026-11-10T09:00:00Z" }));
  await p;
  expect(useItemsStore.getState().items.r?.dueAt).toBe("2026-11-10T09:00:00Z");
});

test("complete rolls back when invoke rejects", async () => {
  useItemsStore.setState({ order: ["a"], items: { a: item({ id: "a" }) } });
  mocked.mockRejectedValueOnce(new Error("boom"));
  await useItemsStore.getState().complete("a");
  expect(useItemsStore.getState().items.a?.status).toBe("inbox");
  expect(useToastsStore.getState().toasts.at(-1)?.message).toBe("boom");
});

describe("undoable delete", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  test("undo within the window restores the card with no server call", () => {
    useItemsStore.setState({
      order: ["a", "b"],
      items: { a: item({ id: "a" }), b: item({ id: "b" }) },
    });
    useItemsStore.getState().remove("a");
    expect(useItemsStore.getState().order).toEqual(["b"]);
    useToastsStore.getState().toasts.at(-1)?.action?.run();
    expect(useItemsStore.getState().order).toEqual(["a", "b"]);
    vi.advanceTimersByTime(6000);
    expect(mocked).not.toHaveBeenCalledWith("items_trash", { id: "a" });
  });

  test("after the window the delete is sent exactly once", () => {
    mocked.mockResolvedValue(undefined);
    useItemsStore.setState({ order: ["a"], items: { a: item({ id: "a" }) } });
    useItemsStore.getState().remove("a");
    useItemsStore.getState().remove("a");
    vi.advanceTimersByTime(5000);
    const deletes = mocked.mock.calls.filter((c) => c[0] === "items_trash");
    expect(deletes).toEqual([["items_trash", { id: "a" }]]);
  });
});

test("refreshCounts updates counts", async () => {
  mocked.mockResolvedValueOnce({ ...EMPTY_COUNTS, inbox: 3, byProject: { p1: 2 } });
  await useItemsStore.getState().refreshCounts();
  expect(useItemsStore.getState().counts.inbox).toBe(3);
});
