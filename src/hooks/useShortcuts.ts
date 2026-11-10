import { useEffect, useState } from "react";
import { onShortcutsChanged, shortcutsGet, type ShortcutMap } from "../api/desktop";
import { DEFAULT_SHORTCUTS, formatAccelerator } from "../lib/shortcuts";

/** Current global shortcuts (defaults until loaded), kept live across windows. */
export function useShortcuts(): ShortcutMap {
  const [map, setMap] = useState<ShortcutMap>(DEFAULT_SHORTCUTS);
  useEffect(() => {
    let alive = true;
    shortcutsGet()
      .then((m) => alive && m && setMap(m))
      .catch(() => undefined);
    const un = onShortcutsChanged(setMap);
    return () => {
      alive = false;
      void un.then((f) => f());
    };
  }, []);
  return map;
}

/** Printable labels, e.g. `{ voice: "⌥ Space", capture: "⌘⇧N" }`. */
export function useShortcutLabels(): Record<keyof ShortcutMap, string> {
  const map = useShortcuts();
  return { voice: formatAccelerator(map.voice), capture: formatAccelerator(map.capture) };
}
