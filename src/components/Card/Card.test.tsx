import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Item } from "../../api/types";
import { Card } from "./Card";

const now = new Date(2026, 10, 9, 13, 50);
const local = (d: number, h: number, m = 0) => new Date(2026, 10, d, h, m).toISOString();

const base = (over: Partial<Item>): Item => ({
  id: "i1",
  kind: "task",
  title: "Send revised deck to Marta",
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

test("task shows checkbox, deadline, priority and project", () => {
  render(
    <Card
      now={now}
      item={base({ dueAt: local(12, 18), priority: "high" })}
      project={{ id: "p", name: "Spring launch", color: "#79B8F5", boardId: "b", createdAt: "" }}
    />,
  );
  expect(screen.getByRole("checkbox", { name: /mark complete/i })).toBeInTheDocument();
  expect(screen.getByText("Thu 12 · 18:00")).toBeInTheDocument();
  expect(screen.getByText("High")).toBeInTheDocument();
  expect(screen.getByText("Spring launch")).toBeInTheDocument();
});

test("reminder leads with the time and a relative countdown", () => {
  render(
    <Card now={now} item={base({ kind: "reminder", title: "Call", remindAt: local(9, 16) })} />,
  );
  expect(screen.getByText("Today, 16:00")).toBeInTheDocument();
  expect(screen.getByText("in 2h 10m")).toBeInTheDocument();
});

test("note shows a preview and tags", () => {
  render(
    <Card
      now={now}
      item={base({
        kind: "note",
        title: "Interview",
        body: "She keeps reminders",
        tags: ["research"],
      })}
    />,
  );
  expect(screen.getByText("She keeps reminders")).toBeInTheDocument();
  expect(screen.getByText("#research")).toBeInTheDocument();
});

test("idea shows linked count", () => {
  render(<Card now={now} item={base({ kind: "idea", title: "Dim the orb", linkCount: 3 })} />);
  expect(screen.getByText("3 linked")).toBeInTheDocument();
});

test("project card shows progress", () => {
  render(
    <Card
      now={now}
      item={base({ kind: "project_ref", title: "Spring launch" })}
      progress={{ done: 8, total: 13 }}
    />,
  );
  expect(screen.getByText("8/13")).toBeInTheDocument();
});

test("event shows date block and time range", () => {
  render(
    <Card
      now={now}
      item={base({
        kind: "event",
        title: "Design review",
        dueAt: local(11, 14, 30),
        endsAt: local(11, 15, 30),
      })}
    />,
  );
  expect(screen.getByText("WED")).toBeInTheDocument();
  expect(screen.getByText("14:30 – 15:30")).toBeInTheDocument();
});

test("completed state and checkbox toggle", async () => {
  const onToggle = vi.fn();
  render(<Card now={now} item={base({ status: "done" })} onToggle={onToggle} />);
  expect(screen.getByRole("article")).toHaveAttribute("data-done", "true");
  await userEvent.click(screen.getByRole("checkbox", { name: /not done/i }));
  expect(onToggle).toHaveBeenCalledWith("i1");
});

test("double click opens, click selects, actions do not select", async () => {
  const onOpen = vi.fn();
  const onSelect = vi.fn();
  const onDelete = vi.fn();
  render(
    <Card now={now} item={base({})} onOpen={onOpen} onSelect={onSelect} onDelete={onDelete} />,
  );
  await userEvent.dblClick(screen.getByRole("article"));
  expect(onOpen).toHaveBeenCalledWith("i1");
  onSelect.mockClear();
  await userEvent.click(screen.getByRole("button", { name: "Delete" }));
  expect(onDelete).toHaveBeenCalledWith("i1");
  expect(onSelect).not.toHaveBeenCalled();
});

test("capturing placeholder shows the reading state instead of the body", () => {
  render(<Card now={now} item={base({ kind: "note", title: "call marta", capturing: true })} />);
  expect(screen.getAllByText("Reading…").length).toBeGreaterThan(0);
  expect(screen.getByRole("article")).toHaveAttribute("data-capturing", "true");
});

test("hint chips show check-date and new-project suggestions", () => {
  render(
    <Card
      now={now}
      item={base({ hint: { datePhrase: "the first", suggestedProject: "Kitchen" } })}
    />,
  );
  expect(screen.getByText("Check date · the first")).toBeInTheDocument();
  expect(screen.getByText("New project? Kitchen")).toBeInTheDocument();
});
