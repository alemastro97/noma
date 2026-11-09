import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
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
