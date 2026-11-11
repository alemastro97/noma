import { useEffect } from "react";
import { onOpenPalette } from "../api/presence";
import { useUiStore } from "../stores/ui";

/** Global shortcuts that land in the main window: Search opens the command palette. */
export function useDesktopEvents(): void {
  useEffect(() => {
    const un = onOpenPalette(() => useUiStore.getState().setPaletteOpen(true));
    return () => {
      void un.then((f) => f()).catch(() => undefined);
    };
  }, []);
}
