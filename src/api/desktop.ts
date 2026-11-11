import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Action } from "../lib/shortcuts";
import type { PanelName, ReminderToast, VoiceState } from "./types";

// ── Voice ───────────────────────────────────────────────────────────────

export const voiceStart = () => invoke<void>("voice_start");
/** Stop listening and save now. */
export const voiceStop = () => invoke<void>("voice_stop");
export const voiceCancel = () => invoke<void>("voice_cancel");
/** Stop listening and return the transcript for editing. */
export const voiceEdit = () => invoke<string | null>("voice_edit");
export const voiceCurrent = () => invoke<VoiceState>("voice_current");

export function onVoiceState(cb: (s: VoiceState) => void): Promise<UnlistenFn> {
  return listen<VoiceState>("voice:state", (e) => cb(e.payload));
}

export function onVoiceLevel(cb: (level: number) => void): Promise<UnlistenFn> {
  return listen<number>("voice:level", (e) => cb(e.payload));
}

export function onVoicePartial(cb: (text: string) => void): Promise<UnlistenFn> {
  return listen<string>("voice:partial", (e) => cb(e.payload));
}

// ── Panels ──────────────────────────────────────────────────────────────

export const panelShow = (name: PanelName) => invoke<void>("panel_show", { name });
export const panelHide = (name: PanelName) => invoke<void>("panel_hide", { name });
export const panelToggle = (name: PanelName) => invoke<void>("panel_toggle", { name });
export const mainShow = () => invoke<void>("main_show");

// ── Reminders ───────────────────────────────────────────────────────────

export const remindersActive = () => invoke<ReminderToast[]>("reminders_active");
export const reminderDismiss = (id: string) => invoke<void>("reminder_dismiss", { id });
export const reminderDone = (itemId: string) => invoke<void>("reminder_done", { itemId });
export function reminderSnooze(itemId: string, opts: { minutes?: number; tomorrow?: boolean }) {
  return invoke<void>("reminder_snooze", { itemId, ...opts });
}

export function onRemindersChanged(cb: (toasts: ReminderToast[]) => void): Promise<UnlistenFn> {
  return listen<ReminderToast[]>("reminders:changed", (e) => cb(e.payload));
}

// ── UI settings (ui., onboarding., orb., reminders., voice.) ───────────

export const settingGet = (key: string) => invoke<string | null>("settings_get", { key });
export const settingSet = (key: string, value: string) =>
  invoke<void>("settings_set", { key, value });

// ── Global shortcuts ────────────────────────────────────────────────────

export type ShortcutMap = Record<Action, string>;

export const shortcutsGet = () => invoke<ShortcutMap>("shortcuts_get");
/** Rejects with a readable message if invalid, duplicated or owned by another app. */
export const shortcutsSet = (action: Action, accelerator: string) =>
  invoke<ShortcutMap>("shortcuts_set", { action, accelerator });
export const shortcutsReset = (action: Action) =>
  invoke<ShortcutMap>("shortcuts_reset", { action });
/** Frees the shortcuts while the recorder listens, so pressing one doesn't fire it. */
export const shortcutsSuspend = () => invoke<void>("shortcuts_suspend");
export const shortcutsResume = () => invoke<void>("shortcuts_resume");

export function onShortcutsChanged(cb: (map: ShortcutMap) => void): Promise<UnlistenFn> {
  return listen<ShortcutMap>("shortcuts:changed", (e) => cb(e.payload));
}

// ── Open at login ───────────────────────────────────────────────────────

export const autostartGet = () => invoke<boolean>("autostart_get");
export const autostartSet = (enabled: boolean) => invoke<boolean>("autostart_set", { enabled });
