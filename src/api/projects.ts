import { invoke } from "@tauri-apps/api/core";
import type { Progress, Project } from "./types";

export function listProjects(): Promise<Project[]> {
  return invoke<Project[]>("projects_list");
}

export function createProject(name: string, color: string): Promise<Project> {
  return invoke<Project>("projects_create", { name, color });
}

export function getProgress(): Promise<Record<string, Progress>> {
  return invoke<Record<string, Progress>>("projects_progress");
}
