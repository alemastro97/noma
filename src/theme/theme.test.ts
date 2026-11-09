import { applyTheme, initTheme, setTheme, THEME_STORAGE_KEY } from "./theme";

type Listener = (e: { matches: boolean }) => void;

function mockMatchMedia(initialMatches: boolean) {
  const listeners = new Set<Listener>();
  const mql = {
    matches: initialMatches,
    media: "(prefers-color-scheme: light)",
    addEventListener: (_: "change", cb: Listener) => listeners.add(cb),
    removeEventListener: (_: "change", cb: Listener) => listeners.delete(cb),
  };
  window.matchMedia = vi.fn(() => mql as unknown as MediaQueryList);
  return {
    flip(matches: boolean) {
      mql.matches = matches;
      listeners.forEach((cb) => cb({ matches }));
    },
    listenerCount: () => listeners.size,
  };
}

beforeEach(() => {
  localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
});

test("applyTheme('light') sets data-theme=light", () => {
  applyTheme("light");
  expect(document.documentElement.dataset.theme).toBe("light");
});

test("applyTheme('dark') sets data-theme=dark", () => {
  applyTheme("dark");
  expect(document.documentElement.dataset.theme).toBe("dark");
});

test("applyTheme('system') resolves from matchMedia", () => {
  mockMatchMedia(true);
  applyTheme("system");
  expect(document.documentElement.dataset.theme).toBe("light");
});

test("system theme tracks matchMedia change", () => {
  const media = mockMatchMedia(false);
  const stop = initTheme();
  expect(document.documentElement.dataset.theme).toBe("dark");
  media.flip(true);
  expect(document.documentElement.dataset.theme).toBe("light");
  stop();
  expect(media.listenerCount()).toBe(0);
});

test("setTheme persists to localStorage key noma.theme", () => {
  mockMatchMedia(false);
  setTheme("light");
  expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe("light");
  expect(THEME_STORAGE_KEY).toBe("noma.theme");
  expect(document.documentElement.dataset.theme).toBe("light");
});

test("initTheme reads the saved theme", () => {
  mockMatchMedia(false);
  localStorage.setItem(THEME_STORAGE_KEY, "light");
  initTheme();
  expect(document.documentElement.dataset.theme).toBe("light");
});
