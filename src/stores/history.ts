/**
 * Global undo / redo for the main window: an in-memory stack of entries, each
 * carrying its own inverse. Entries replay through the regular store actions
 * and commands, so the backend stays the single source of truth.
 *
 * Rule for `undo` / `redo` closures: call the recording store action
 * synchronously (before any `await`), so the replay guard suppresses it.
 */
import { workspaceCopy } from "../copy";
import { useToastsStore } from "./toasts";

export type HistoryAction = keyof typeof workspaceCopy.actions;

export interface HistoryEntry {
  action: HistoryAction;
  undo: () => unknown;
  redo: () => unknown;
}

const LIMIT = 100;
let past: HistoryEntry[] = [];
let future: HistoryEntry[] = [];
let replaying = 0;
let batch: HistoryEntry[] | null = null;

export function isReplaying(): boolean {
  return replaying > 0;
}

/** Adds an entry (ignored while an undo / redo is replaying). Clears redo. */
export function record(entry: HistoryEntry): void {
  if (replaying > 0) return;
  if (batch) {
    batch.push(entry);
    return;
  }
  past.push(entry);
  if (past.length > LIMIT) past.shift();
  future = [];
}

function combine(action: HistoryAction, entries: HistoryEntry[]): HistoryEntry {
  return {
    action,
    // All replays start synchronously so the guard covers every one of them.
    undo: () => Promise.all([...entries].reverse().map((e) => e.undo())),
    redo: () => Promise.all(entries.map((e) => e.redo())),
  };
}

/** Runs `fn` and records everything it records as one entry. */
export function batchHistory<T>(action: HistoryAction, fn: () => T): T {
  if (batch || replaying > 0) return fn();
  batch = [];
  try {
    return fn();
  } finally {
    const entries = batch;
    batch = null;
    if (entries.length === 1) record(entries[0]!);
    else if (entries.length > 1) record(combine(action, entries));
  }
}

async function replay(fn: () => unknown): Promise<void> {
  replaying++;
  let pending: unknown;
  try {
    pending = fn();
  } finally {
    replaying--;
  }
  await pending;
}

function toast(message: string) {
  useToastsStore.getState().push({ message }, 2500);
}

export async function undo(): Promise<boolean> {
  const entry = past.pop();
  if (!entry) {
    toast(workspaceCopy.nothingToUndo);
    return false;
  }
  future.push(entry);
  toast(workspaceCopy.undone(workspaceCopy.actions[entry.action]));
  await replay(entry.undo);
  return true;
}

export async function redo(): Promise<boolean> {
  const entry = future.pop();
  if (!entry) {
    toast(workspaceCopy.nothingToRedo);
    return false;
  }
  past.push(entry);
  toast(workspaceCopy.redone(workspaceCopy.actions[entry.action]));
  await replay(entry.redo);
  return true;
}

export function historySize(): { past: number; future: number } {
  return { past: past.length, future: future.length };
}

/** Tests only. */
export function resetHistory(): void {
  past = [];
  future = [];
  batch = null;
  replaying = 0;
}
