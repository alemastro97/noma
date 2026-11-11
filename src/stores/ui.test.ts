import { AVAILABLE_VIEW_MODES, UI_STORAGE_KEY, useUiStore } from "./ui";

beforeEach(() => {
  localStorage.clear();
  useUiStore.setState({ target: { kind: "inbox" }, viewMode: "canvas", sidebarCollapsed: false });
});

test("default target is inbox and view mode canvas", () => {
  const s = useUiStore.getState();
  expect(s.target).toEqual({ kind: "inbox" });
  expect(s.viewMode).toBe("canvas");
  expect(s.sidebarCollapsed).toBe(false);
});

test("setTarget switches the active target", () => {
  useUiStore.getState().setTarget({ kind: "project", id: "p1" });
  expect(useUiStore.getState().target).toEqual({ kind: "project", id: "p1" });
});

test("setViewMode accepts available modes", () => {
  useUiStore.getState().setViewMode("list");
  expect(useUiStore.getState().viewMode).toBe("list");
  expect(AVAILABLE_VIEW_MODES).toEqual([
    "canvas",
    "kanban",
    "list",
    "graph",
    "calendar",
    "timeline",
  ]);
});

test("setViewMode('calendar') switches to the calendar", () => {
  useUiStore.getState().setViewMode("calendar");
  expect(useUiStore.getState().viewMode).toBe("calendar");
});

test("toggleSidebar flips and persists to localStorage noma.ui", () => {
  useUiStore.getState().toggleSidebar();
  expect(useUiStore.getState().sidebarCollapsed).toBe(true);
  expect(UI_STORAGE_KEY).toBe("noma.ui");
  const saved = JSON.parse(localStorage.getItem(UI_STORAGE_KEY) ?? "{}");
  expect(saved.state.sidebarCollapsed).toBe(true);
});
