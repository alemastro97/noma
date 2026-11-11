import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Iso, Item } from "./types";

/** Time features: custom snooze, "before" reminders, habits, focus timer, evening recap. */

export interface HabitStats {
  /** Occurrences done on time in a row; 0 while overdue. */
  streak: number;
  /** Last 7 local days, oldest first. */
  last7: boolean[];
}

export interface Habit extends HabitStats {
  item: Item;
}

export interface FocusStats {
  sessions: number;
  minutes: number;
}

export interface ItemTime {
  /** Minutes before the due time. */
  offsets: number[];
  habit: boolean;
  habitStats: HabitStats | null;
  focus: FocusStats;
}

export interface FocusState {
  itemId: string;
  title: string;
  startedAt: Iso;
  endsAt: Iso;
  minutes: number;
}

export const reminderSnoozeUntil = (itemId: string, until: Iso) =>
  invoke<void>("reminder_snooze_until", { itemId, until });
export const recapOpen = () => invoke<void>("recap_open");
export const timeItemInfo = (itemId: string) => invoke<ItemTime>("time_item_info", { itemId });
export const reminderOffsetsSet = (itemId: string, minutes: number[]) =>
  invoke<number[]>("reminder_offsets_set", { itemId, minutes });
export const habitSet = (itemId: string, on: boolean) => invoke<void>("habit_set", { itemId, on });
export const habitsList = () => invoke<Habit[]>("habits_list");
export const focusCurrent = () => invoke<FocusState | null>("focus_current");
export const focusStart = (itemId: string, minutes?: number) =>
  invoke<FocusState>("focus_start", { itemId, minutes });
export const focusStop = () => invoke<void>("focus_stop");

export function onFocusChanged(cb: (focus: FocusState | null) => void): Promise<UnlistenFn> {
  return listen<FocusState | null>("focus:changed", (e) => cb(e.payload));
}

/** Toast ids the time features use (see `src-tauri/src/timekeeping.rs`). */
export const RECAP_TOAST = "recap";
export const isFocusToast = (id: string) => id.startsWith("focus:");
export const isBeforeToast = (id: string) => id.startsWith("pre:");
