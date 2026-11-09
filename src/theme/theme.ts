export type Theme = "dark" | "light" | "system";

export const THEME_STORAGE_KEY = "noma.theme";

const LIGHT_QUERY = "(prefers-color-scheme: light)";

function systemPrefersLight(): boolean {
  return typeof window !== "undefined" && !!window.matchMedia?.(LIGHT_QUERY).matches;
}

function resolve(theme: Theme): "dark" | "light" {
  if (theme === "system") return systemPrefersLight() ? "light" : "dark";
  return theme;
}

export function applyTheme(theme: Theme, root: HTMLElement = document.documentElement): void {
  root.dataset.theme = resolve(theme);
}

export function readTheme(): Theme {
  try {
    const saved = localStorage.getItem(THEME_STORAGE_KEY);
    if (saved === "dark" || saved === "light" || saved === "system") return saved;
  } catch {
    // storage unavailable: fall through to default
  }
  return "system";
}

export function setTheme(theme: Theme): void {
  try {
    localStorage.setItem(THEME_STORAGE_KEY, theme);
  } catch {
    // storage unavailable: still apply for this session
  }
  applyTheme(theme);
}

/**
 * Applies the saved theme and keeps it current: follows the OS whenever the
 * saved theme is "system" (even if chosen later), and follows changes made in
 * other windows (the orb window shares localStorage). Returns an unsubscribe.
 */
export function initTheme(): () => void {
  applyTheme(readTheme());
  if (typeof window === "undefined") return () => undefined;

  const mql = window.matchMedia?.(LIGHT_QUERY);
  const onSystemChange = () => {
    if (readTheme() === "system") applyTheme("system");
  };
  const onStorage = (e: StorageEvent) => {
    if (e.key === THEME_STORAGE_KEY) applyTheme(readTheme());
  };
  mql?.addEventListener("change", onSystemChange);
  window.addEventListener("storage", onStorage);
  return () => {
    mql?.removeEventListener("change", onSystemChange);
    window.removeEventListener("storage", onStorage);
  };
}
