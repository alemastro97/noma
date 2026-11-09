import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Header } from "./Header";
import { useUiStore } from "../../stores/ui";

beforeEach(() => {
  useUiStore.setState({ target: { kind: "inbox" }, viewMode: "canvas", sidebarCollapsed: false });
});

test("Calendar and Timeline buttons are disabled and labelled Soon", () => {
  render(<Header title="Inbox" count={4} />);
  const calendar = screen.getByRole("button", { name: /calendar/i });
  const timeline = screen.getByRole("button", { name: /timeline/i });
  expect(calendar).toBeDisabled();
  expect(timeline).toBeDisabled();
  expect(calendar).toHaveAccessibleName(/soon/i);
});

test("active view mode has aria-pressed=true and clicking switches", async () => {
  render(<Header title="Inbox" count={4} />);
  expect(screen.getByRole("button", { name: /^canvas$/i })).toHaveAttribute("aria-pressed", "true");
  await userEvent.click(screen.getByRole("button", { name: /^list$/i }));
  expect(useUiStore.getState().viewMode).toBe("list");
  expect(screen.getByRole("button", { name: /^list$/i })).toHaveAttribute("aria-pressed", "true");
});

test("shows title, card count and Idle AI status with an orb", () => {
  render(<Header title="Spring launch" count={13} color="#79B8F5" />);
  expect(screen.getByRole("heading", { level: 1, name: "Spring launch" })).toBeInTheDocument();
  expect(screen.getByText("13 cards")).toBeInTheDocument();
  expect(screen.getByText("Idle")).toBeInTheDocument();
  expect(screen.getByRole("img", { name: /ai status/i })).toHaveClass("s-idle");
});

test("singular card count", () => {
  render(<Header title="Inbox" count={1} />);
  expect(screen.getByText("1 card")).toBeInTheDocument();
});

test("has search with ⌘K hint and a Capture button", () => {
  render(<Header title="Inbox" count={0} />);
  expect(screen.getByRole("searchbox", { name: /search/i })).toBeInTheDocument();
  expect(screen.getByText("⌘K")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /capture/i })).toBeInTheDocument();
});
