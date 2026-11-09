import { filterCommands, type Command } from "./commands";

const cmd = (label: string, keywords?: string): Command => ({
  id: label,
  label,
  section: "actions",
  keywords,
  run: () => undefined,
});

const all = [
  cmd("Go to Inbox"),
  cmd("New task"),
  cmd("New note"),
  cmd("Theme: Light", "appearance"),
];

test("empty query returns everything in order", () => {
  expect(filterCommands(all, "  ")).toEqual(all);
});

test("prefix matches rank first, then word starts, then substrings", () => {
  expect(filterCommands(all, "new").map((c) => c.label)).toEqual(["New task", "New note"]);
  expect(filterCommands(all, "inbox").map((c) => c.label)).toEqual(["Go to Inbox"]);
});

test("keywords and subsequences match", () => {
  expect(filterCommands(all, "appear").map((c) => c.label)).toEqual(["Theme: Light"]);
  expect(filterCommands(all, "nwtsk").map((c) => c.label)).toEqual(["New task"]);
  expect(filterCommands(all, "zzz")).toEqual([]);
});
