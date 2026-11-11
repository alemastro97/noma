import { useEffect, useState } from "react";
import { onShortcutsChanged, shortcutsGet, type ShortcutMap } from "../api/desktop";
import { DEFAULT_SHORTCUTS, formatAccelerator } from "../lib/shortcuts";

/** Current global shortcuts (defaults until loaded), kept live across windows. */
export function useShortcuts(): ShortcutMap {
  const [map, setMap] = useState<ShortcutMap>(DEFAULT_SHORTCUTS);
  useEffect(() => {
    let alive = true;
    shortcutsGet()
      .then((m) => alive && m && setMap({ ...DEFAULT_SHORTCUTS, ...m }))
      .catch(() => undefined);
    const un = onShortcutsChanged((m) => setMap({ ...DEFAULT_SHORTCUTS, ...m }));
    return () => {
      alive = false;
      void un.then((f) => f());
    };
  }, []);
  return map;
}

/** Printable labels, e.g. `{ voice: "⌥ Space", capture: "⌘⇧N" }` ("" when unset). */
export function useShortcutLabels(): Record<keyof ShortcutMap, string> {
  const map = useShortcuts();
  return Object.fromEntries(
    Object.entries(map).map(([k, v]) => [k, v ? formatAccelerator(v) : ""]),
  ) as Record<keyof ShortcutMap, string>;
}
