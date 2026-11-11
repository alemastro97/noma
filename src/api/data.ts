import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ── Restore ─────────────────────────────────────────────────────────────

export interface Snapshot {
  name: string;
  at: string;
  sizeBytes: number;
}

export const backupList = () => invoke<Snapshot[]>("backup_list");
/** Takes a fresh snapshot, then swaps the database for `name`. */
export const backupRestore = (name: string) => invoke<void>("backup_restore", { name });

export function onDataRestored(cb: () => void): Promise<UnlistenFn> {
  return listen("data:restored", () => cb());
}

// ── Import ──────────────────────────────────────────────────────────────

export type TextFormat = "markdown" | "text" | "todoist";
export type AppleSource = "notes" | "reminders";

export interface ImportSummary {
  created: number;
  skipped: number;
  ids: string[];
}

/** Error code returned when macOS denied Automation access to Notes/Reminders. */
export const AUTOMATION_DENIED = "automation-denied";

export const importText = (format: TextFormat, content: string, projectId: string | null) =>
  invoke<ImportSummary>("import_text", { format, content, projectId });
export const importAppleSources = (source: AppleSource) =>
  invoke<string[]>("import_apple_sources", { source });
export const importApple = (source: AppleSource, name: string, projectId: string | null) =>
  invoke<ImportSummary>("import_apple", { source, name, projectId });

// ── Folder auto-export ──────────────────────────────────────────────────

/** Native folder picker; null when cancelled. */
export const pickFolder = (prompt: string) => invoke<string | null>("pick_folder", { prompt });
export interface MirrorConfig {
  enabled: boolean;
  folder: string | null;
  /** Where noma.md was just written by this call, if it was. */
  written: string | null;
}

export const mirrorGet = () => invoke<MirrorConfig>("mirror_get");
/** Changes the switch and/or folder, then writes noma.md right away. */
export const mirrorSet = (patch: { enabled?: boolean; folder?: string }) =>
  invoke<MirrorConfig>("mirror_set", patch);

// ── CLI and MCP ─────────────────────────────────────────────────────────

export interface CliInfo {
  script: string;
  path: string;
  installed: boolean;
}

export const cliInfo = () => invoke<CliInfo>("cli_info");
export const cliInstall = () => invoke<CliInfo>("cli_install");

export interface McpInfo {
  command: string;
  exists: boolean;
  dbPath: string;
}

export const mcpInfo = () => invoke<McpInfo>("mcp_info");

// ── Capture history and stats ───────────────────────────────────────────

export type CaptureStatus = "pending" | "processing" | "done" | "failed";

export interface CaptureRow {
  id: string;
  source: "voice" | "text";
  rawText: string;
  status: CaptureStatus;
  latencyMs: number | null;
  createdAt: string;
  itemId: string | null;
  itemTitle: string | null;
}

export interface WeekStats {
  /** Monday of the week, YYYY-MM-DD. */
  start: string;
  captures: number;
  completed: number;
}

export interface Stats {
  weeks: WeekStats[];
  avgLatencyMs: number | null;
}

export const capturesRecent = (limit?: number) =>
  invoke<CaptureRow[]>("captures_recent", { limit });
export const capturesRetry = (id: string) => invoke<void>("captures_retry", { id });
export const statsGet = () => invoke<Stats>("stats_get");
