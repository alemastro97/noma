import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type OrbSize = "s" | "m" | "l";
export type OrbStyle = "lively" | "calm" | "faded";

/** What the floating orb and the menu bar show (`presence:changed`). */
export interface PresenceState {
  /** A reminder is due within 15 minutes or overdue (never during quiet hours). */
  attention: boolean;
  /** Inside the daily quiet window: reminders wait, the orb stays still. */
  quiet: boolean;
  todayLeft: number;
  orbSize: OrbSize;
  orbStyle: OrbStyle;
}

export const DEFAULT_PRESENCE: PresenceState = {
  attention: false,
  quiet: false,
  todayLeft: 0,
  orbSize: "m",
  orbStyle: "lively",
};

export const presenceGet = () => invoke<PresenceState>("presence_get");
/** Main window on that card (from the orb panel's search). */
export const presenceOpenItem = (itemId: string) => invoke<void>("presence_open_item", { itemId });

export function onPresence(cb: (s: PresenceState) => void): Promise<UnlistenFn> {
  return listen<PresenceState>("presence:changed", (e) => cb(e.payload));
}

/** The Search shortcut: main window with the command palette. */
export function onOpenPalette(cb: () => void): Promise<UnlistenFn> {
  return listen("ui:open-palette", () => cb());
}
