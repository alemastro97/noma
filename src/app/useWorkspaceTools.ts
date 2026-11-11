import { useEffect } from "react";
import { bulkComplete, bulkDelete } from "../stores/bulk";
import { useFiltersStore } from "../stores/filters";
import { redo, undo } from "../stores/history";
import { installPointerModifiers, useSelectionStore, visibleOrder } from "../stores/selection";
import { isTypingTarget, useUiStore } from "../stores/ui";

const IS_MAC = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform ?? "");

/**
 * Workspace keyboard, registered in the capture phase so it runs before the
 * app-wide hotkeys: ⌘Z undo · ⇧⌘Z redo · ⌘F filter bar · ⌘4 graph · ⌘A select
 * all visible cards · with several cards selected, ⌫ deletes and Space
 * completes all of them.
 */
export function useWorkspaceTools(): void {
  useEffect(() => {
    installPointerModifiers();
    void useFiltersStore.getState().load();

    const onKey = (e: KeyboardEvent) => {
      const ui = useUiStore.getState();
      const mod = IS_MAC ? e.metaKey : e.ctrlKey;
      const key = e.key.toLowerCase();
      const typing = isTypingTarget(e.target);
      const overlayOpen = ui.paletteOpen || ui.quickAdd.open;
      const inWorkspace = ui.target.kind !== "settings";
      const stop = () => {
        e.preventDefault();
        e.stopImmediatePropagation();
      };

      if (mod && key === "f" && inWorkspace && !overlayOpen) {
        stop();
        const filters = useFiltersStore.getState();
        filters.setOpen(!filters.open);
        return;
      }
      if (mod && key === "4" && !overlayOpen) {
        stop();
        ui.setViewMode("graph");
        return;
      }
      // Text fields keep their own undo.
      if (typing || overlayOpen) return;

      if (mod && key === "z") {
        stop();
        void (e.shiftKey ? redo() : undo());
        return;
      }
      if (mod && key === "a" && inWorkspace) {
        const ids = visibleOrder();
        if (ids.length === 0) return;
        stop();
        useSelectionStore.getState().set(ids, ids[0]);
        ui.select(ids[ids.length - 1]!);
        return;
      }

      const ids = useSelectionStore.getState().ids;
      if (ids.length < 2) return;
      if ((key === "backspace" || key === "delete") && !ui.detailId) {
        stop();
        bulkDelete(ids);
      } else if (key === " " && !(e.target instanceof HTMLButtonElement)) {
        stop();
        bulkComplete(ids);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, []);
}
