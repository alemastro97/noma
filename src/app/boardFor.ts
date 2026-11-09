import type { NavTarget } from "../stores/ui";

/** The board a target shows on the canvas: a project's board or a personal board. */
export function boardIdFor(
  target: NavTarget,
  projectBoards: Record<string, string>,
): string | null {
  if (target.kind === "board") return target.id;
  if (target.kind === "project") return projectBoards[target.id] ?? null;
  return null;
}
