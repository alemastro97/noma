import { create } from "zustand";
import * as boardsApi from "../api/boards";
import * as projectsApi from "../api/projects";
import type { Board, Progress, Project } from "../api/types";
import { toastError } from "./toasts";

export interface ProjectsState {
  projects: Project[];
  boards: Board[];
  progress: Record<string, Progress>;
  load: () => Promise<void>;
  createProject: (name: string, color: string) => Promise<Project | null>;
  createBoard: (name: string) => Promise<Board | null>;
}

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export const useProjectsStore = create<ProjectsState>()((set, get) => ({
  projects: [],
  boards: [],
  progress: {},

  async load() {
    try {
      const [projects, boards, progress] = await Promise.all([
        projectsApi.listProjects(),
        boardsApi.listBoards(),
        projectsApi.getProgress(),
      ]);
      // Same data → same objects, so cards and the sidebar do not re-render.
      const prev = get();
      const keep = <T>(old: T, next: T): T =>
        JSON.stringify(old) === JSON.stringify(next) ? old : next;
      const next = {
        projects: keep(prev.projects, projects ?? []),
        boards: keep(prev.boards, boards ?? []),
        progress: keep(prev.progress, progress ?? {}),
      };
      if (
        next.projects !== prev.projects ||
        next.boards !== prev.boards ||
        next.progress !== prev.progress
      )
        set(next);
    } catch (err) {
      toastError(message(err));
    }
  },

  async createProject(name, color) {
    try {
      const project = await projectsApi.createProject(name, color);
      await get().load();
      return project;
    } catch (err) {
      toastError(message(err));
      return null;
    }
  },

  async createBoard(name) {
    try {
      const board = await boardsApi.createBoard(name);
      set((s) => ({ boards: [...s.boards, board] }));
      return board;
    } catch (err) {
      toastError(message(err));
      return null;
    }
  },
}));

/** Personal boards only — project boards are reached through their project. */
export function personalBoards(boards: Board[]): Board[] {
  return boards.filter((b) => b.kind === "personal");
}
