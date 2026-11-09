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
 * Applies the saved theme and keeps following the OS while the theme is "system".
 * Returns an unsubscribe function.
 */
export function initTheme(): () => void {
  const theme = readTheme();
  applyTheme(theme);
  if (theme !== "system" || typeof window === "undefined" || !window.matchMedia) {
    return () => undefined;
  }
  const mql = window.matchMedia(LIGHT_QUERY);
  const onChange = () => {
    if (readTheme() === "system") applyTheme("system");
  };
  mql.addEventListener("change", onChange);
  return () => mql.removeEventListener("change", onChange);
}
