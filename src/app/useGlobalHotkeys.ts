import { useEffect } from "react";
import { useItemsStore } from "../stores/items";
import { isTypingTarget, useUiStore } from "../stores/ui";

const IS_MAC = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform ?? "");

/**
 * App-wide keyboard: ⌘K palette · ⌘N / N quick add · ⌘1-3 views · ⌘\ is in the
 * sidebar. On a selected card: E / ↵ open · ⌫ delete · ⌘D duplicate · Space
 * toggle complete · Esc deselect / close.
 */
export function useGlobalHotkeys(): void {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const ui = useUiStore.getState();
      const mod = IS_MAC ? e.metaKey : e.ctrlKey;
      const typing = isTypingTarget(e.target);
      const key = e.key.toLowerCase();
      const overlayOpen = ui.paletteOpen || ui.quickAdd.open;

      if (mod && key === "k") {
        e.preventDefault();
        ui.setPaletteOpen(!ui.paletteOpen);
        return;
      }
      if (mod && key === "n" && !e.shiftKey) {
        e.preventDefault();
        ui.openQuickAdd();
        return;
      }
      if (mod && ["1", "2", "3"].includes(key)) {
        e.preventDefault();
        ui.setViewMode((["canvas", "kanban", "list"] as const)[Number(key) - 1]!);
        return;
      }
      if (overlayOpen) return;

      if (key === "escape") {
        if (ui.detailId) ui.closeDetail();
        else if (ui.selectedId) ui.select(null);
        return;
      }
      if (typing) return;

      if (key === "n" && !mod && !e.altKey) {
        e.preventDefault();
        ui.openQuickAdd();
        return;
      }

      const id = ui.selectedId;
      if (!id || !useItemsStore.getState().items[id]) return;
      const items = useItemsStore.getState();
      if ((key === "e" || key === "enter") && !mod) {
        e.preventDefault();
        ui.openDetail(id);
      } else if ((key === "backspace" || key === "delete") && !ui.detailId) {
        e.preventDefault();
        items.remove(id);
        ui.select(null);
      } else if (mod && key === "d") {
        e.preventDefault();
        void items.duplicate(id);
      } else if (key === " " && !(e.target instanceof HTMLButtonElement)) {
        e.preventDefault();
        void items.toggleComplete(id);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
