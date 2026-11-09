import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Sidebar } from "./Sidebar";
import { useUiStore } from "../../stores/ui";

const projects = [
  { id: "p1", name: "Spring launch", color: "#79B8F5" },
  { id: "p2", name: "Studio move", color: "#78DCC0" },
];
const boards = [{ id: "b1", name: "Thinking space" }];
const counts = { inbox: 4, today: 3, upcoming: 7, ideas: 12, notes: 0, byProject: { p1: 13 } };

beforeEach(() => {
  useUiStore.setState({ target: { kind: "inbox" }, viewMode: "canvas", sidebarCollapsed: false });
});

test("clicking Today sets target today and marks it current", async () => {
  render(<Sidebar counts={counts} projects={projects} boards={boards} />);
  await userEvent.click(screen.getByRole("link", { name: /today/i }));
  expect(useUiStore.getState().target).toEqual({ kind: "today" });
  expect(screen.getByRole("link", { name: /today/i })).toHaveAttribute("aria-current", "page");
  expect(screen.getByRole("link", { name: /inbox/i })).not.toHaveAttribute("aria-current");
});

test("renders counts in the nav rows", () => {
  render(<Sidebar counts={counts} projects={projects} boards={boards} />);
  expect(within(screen.getByRole("link", { name: /inbox/i })).getByText("4")).toBeInTheDocument();
  expect(within(screen.getByRole("link", { name: /notes/i })).queryByText("0")).toBeNull();
});

test("renders project rows with color dot and count", () => {
  render(<Sidebar counts={counts} projects={projects} boards={boards} />);
  const row = screen.getByRole("link", { name: /spring launch/i });
  expect(within(row).getByText("13")).toBeInTheDocument();
  expect(row.querySelector("[data-dot]")).toHaveStyle({ background: "#79B8F5" });
});

test("clicking a board sets a board target", async () => {
  render(<Sidebar counts={counts} projects={projects} boards={boards} />);
  await userEvent.click(screen.getByRole("link", { name: /thinking space/i }));
  expect(useUiStore.getState().target).toEqual({ kind: "board", id: "b1" });
});

test("collapsed sidebar hides labels but keeps accessible names", () => {
  useUiStore.setState({ sidebarCollapsed: true });
  render(<Sidebar counts={counts} projects={projects} boards={boards} />);
  const nav = screen.getByRole("complementary");
  expect(nav).toHaveAttribute("data-collapsed", "true");
  expect(screen.getByRole("link", { name: /inbox/i })).toBeInTheDocument();
  expect(screen.queryByText("Projects")).toBeNull();
});

test("collapse button toggles the store", async () => {
  render(<Sidebar counts={counts} projects={projects} boards={boards} />);
  await userEvent.click(screen.getByRole("button", { name: /collapse sidebar/i }));
  expect(useUiStore.getState().sidebarCollapsed).toBe(true);
});
