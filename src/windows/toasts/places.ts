import type { Board, Place, Project } from "../../api/types";
import { desktopCopy } from "../../copy";

export interface PlaceOption {
  key: string;
  name: string;
  color?: string;
  place: Place;
}

/** Everywhere a capture can be moved to, minus where it already is: Inbox, projects, personal boards. */
export function movePlaces(projects: Project[], boards: Board[], current: Place): PlaceOption[] {
  const all: PlaceOption[] = [
    { key: "inbox", name: desktopCopy.inbox, place: { kind: "inbox" } },
    ...projects.map((p) => ({
      key: `project:${p.id}`,
      name: p.name,
      color: p.color,
      place: { kind: "project", id: p.id } as Place,
    })),
    ...boards
      .filter((b) => b.kind === "personal")
      .map((b) => ({
        key: `board:${b.id}`,
        name: b.name,
        place: { kind: "board", id: b.id } as Place,
      })),
  ];
  const here = "id" in current ? `${current.kind}:${current.id}` : "inbox";
  return all.filter((p) => p.key !== here);
}
