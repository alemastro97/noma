import { listen } from "@tauri-apps/api/event";
import { useSyncExternalStore } from "react";
import { settingGet, settingSet } from "../api/desktop";

/** What the user picked in Settings → Appearance. */
export type LanguagePref = "system" | "en" | "it";
/** The language the interface is actually shown in. */
export type Locale = "en" | "it";

export const LANGUAGE_PREFS: LanguagePref[] = ["system", "en", "it"];
/** Backend setting (also read by Rust for the tray menu and notifications). */
export const LANGUAGE_SETTING = "ui.language";
/** Emitted by the backend to every window after the setting changes. */
export const LANGUAGE_EVENT = "ui:language";
/** Local mirror so each window starts in the right language without a flash. */
export const LANGUAGE_STORAGE_KEY = "noma.language";

function isPref(v: unknown): v is LanguagePref {
  return v === "system" || v === "en" || v === "it";
}

/** The system language, reduced to the ones NOMA speaks (Italian, else English). */
export function systemLocale(): Locale {
  if (typeof navigator === "undefined") return "en";
  const first = navigator.languages?.[0] ?? navigator.language ?? "en";
  return first.toLowerCase().startsWith("it") ? "it" : "en";
}

export function resolveLocale(pref: LanguagePref): Locale {
  return pref === "system" ? systemLocale() : pref;
}

export function readLanguagePref(): LanguagePref {
  try {
    const saved = localStorage.getItem(LANGUAGE_STORAGE_KEY);
    if (isPref(saved)) return saved;
  } catch {
    // storage unavailable: fall through to default
  }
  return "system";
}

let pref: LanguagePref = readLanguagePref();
let current: Locale = resolveLocale(pref);
const subscribers = new Set<() => void>();

export function getLocale(): Locale {
  return current;
}

export function getLanguagePref(): LanguagePref {
  return pref;
}

export function subscribeLocale(fn: () => void): () => void {
  subscribers.add(fn);
  return () => subscribers.delete(fn);
}

/** Applies a preference in this window only (no backend write). */
export function applyLanguagePref(next: LanguagePref): void {
  pref = next;
  try {
    localStorage.setItem(LANGUAGE_STORAGE_KEY, next);
  } catch {
    // storage unavailable: still apply for this session
  }
  const locale = resolveLocale(next);
  if (typeof document !== "undefined") document.documentElement.lang = locale;
  current = locale;
  for (const fn of subscribers) fn();
}

/** Saves the preference; the backend tells every other window (and the tray). */
export function setLanguage(next: LanguagePref): void {
  applyLanguagePref(next);
  void settingSet(LANGUAGE_SETTING, next).catch(() => undefined);
}

/** The active locale, re-rendering on change. */
export function useLocale(): Locale {
  return useSyncExternalStore(subscribeLocale, getLocale, getLocale);
}

/** The saved preference, re-rendering on change. */
export function useLanguagePref(): LanguagePref {
  return useSyncExternalStore(subscribeLocale, getLanguagePref, getLanguagePref);
}

let started = false;

/**
 * Applies the saved language and keeps it current: reconciles with the backend
 * setting, follows changes made in other windows (backend event + shared
 * localStorage) and the system language when the preference is "system".
 */
export function initLocale(): void {
  if (typeof window === "undefined") return;
  document.documentElement.lang = current;
  if (started) return;
  started = true;

  void settingGet(LANGUAGE_SETTING)
    .then((v) => {
      if (isPref(v) && v !== pref) applyLanguagePref(v);
    })
    .catch(() => undefined);

  void listen<string>(LANGUAGE_EVENT, (e) => {
    if (isPref(e.payload) && e.payload !== pref) applyLanguagePref(e.payload);
  }).catch(() => undefined);

  window.addEventListener("storage", (e) => {
    if (e.key === LANGUAGE_STORAGE_KEY && isPref(e.newValue) && e.newValue !== pref)
      applyLanguagePref(e.newValue);
  });
  window.addEventListener("languagechange", () => {
    if (pref === "system") applyLanguagePref("system");
  });
}
