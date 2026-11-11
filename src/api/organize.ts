import { invoke } from "@tauri-apps/api/core";
import type { Iso, Item } from "./types";

/** Mirrors `organize::commands::ResortResult`. */
export interface ResortResult {
  checked: number;
  moved: number;
}

/** Mirrors `organize::briefs::ProjectBrief`. */
export interface ProjectBrief {
  projectId: string;
  summary: string | null;
  summaryAt: Iso | null;
  nextAction: string | null;
  nextItemId: string | null;
  nextAt: Iso | null;
}

/** Mirrors `organize::review::Review`. */
export interface Review {
  completed: Item[];
  overdue: Item[];
  forgotten: Item[];
}

export function resortInbox(): Promise<ResortResult> {
  return invoke<ResortResult>("organize_resort_inbox");
}

export function mergeItems(duplicate: string, into: string): Promise<Item> {
  return invoke<Item>("organize_merge", { duplicate, into });
}

export function breakDown(itemId: string): Promise<string[]> {
  return invoke<string[]>("organize_break_down", { itemId });
}

export function getBrief(projectId: string): Promise<ProjectBrief> {
  return invoke<ProjectBrief>("organize_brief_get", { projectId });
}

export function summarizeProject(projectId: string): Promise<ProjectBrief> {
  return invoke<ProjectBrief>("organize_summarize_project", { projectId });
}

export function suggestNextAction(projectId: string): Promise<ProjectBrief> {
  return invoke<ProjectBrief>("organize_next_action", { projectId });
}

export function getReview(): Promise<Review> {
  return invoke<Review>("organize_review");
}

export function reviewSummary(): Promise<string> {
  return invoke<string>("organize_review_summary");
}
