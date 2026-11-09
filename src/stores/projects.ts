import { create } from "zustand";
import * as boardsApi from "../api/boards";
import * as projectsApi from "../api/projects";
import type { Board, Project } from "../api/types";

export interface ProjectsState {
  projects: Project[];
  boards: Board[];
  load: () => Promise<void>;
  createProject: (name: string, color: string) => Promise<Project>;
  createBoard: (name: string) => Promise<Board>;
}

export const useProjectsStore = create<ProjectsState>()((set, get) => ({
  projects: [],
  boards: [],

  async load() {
    const [projects, boards] = await Promise.all([
      projectsApi.listProjects(),
      boardsApi.listBoards(),
    ]);
    set({ projects: projects ?? [], boards: boards ?? [] });
  },

  async createProject(name, color) {
    const project = await projectsApi.createProject(name, color);
    await get().load();
    return project;
  },

  async createBoard(name) {
    const board = await boardsApi.createBoard(name);
    set((s) => ({ boards: [...s.boards, board] }));
    return board;
  },
}));

/** Personal boards only — project boards are reached through their project. */
export function personalBoards(boards: Board[]): Board[] {
  return boards.filter((b) => b.kind === "personal");
}
