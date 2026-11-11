import type { Board, Project } from "../../api/types";
import { movePlaces } from "./places";

const projects = [{ id: "p1", name: "Studio move", color: "#fff", boardId: "pb1" }] as Project[];
const boards = [
  { id: "pb1", name: "Studio move", kind: "project" },
  { id: "b1", name: "Trips 2027", kind: "personal" },
] as Board[];

test("lists Inbox, projects and personal boards, not project boards", () => {
  const names = movePlaces(projects, boards, { kind: "inbox" }).map((p) => p.name);
  expect(names).toEqual(["Studio move", "Trips 2027"]);
});

test("leaves out where the capture already is", () => {
  const keys = movePlaces(projects, boards, { kind: "project", id: "p1" }).map((p) => p.key);
  expect(keys).toEqual(["inbox", "board:b1"]);
});
