import { Spline } from "lucide-react";
import { copy, workspaceCopy, uiCopy } from "../../copy";
import { ALL_VIEW_MODES, AVAILABLE_VIEW_MODES, useUiStore, type ViewMode } from "../../stores/ui";
import styles from "./Header.module.css";

export function ViewSwitcher() {
  const viewMode = useUiStore((s) => s.viewMode);
  const setViewMode = useUiStore((s) => s.setViewMode);

  return (
    <div className={styles.switcher} role="group" aria-label={uiCopy.regions.viewMode}>
      {ALL_VIEW_MODES.map((mode: ViewMode) => {
        const available = AVAILABLE_VIEW_MODES.includes(mode);
        const label = mode === "graph" ? workspaceCopy.viewGraph : copy.viewModes[mode];
        return (
          <button
            key={mode}
            type="button"
            className={styles.mode}
            aria-pressed={viewMode === mode}
            disabled={!available}
            aria-label={available ? undefined : `${label} — ${copy.soon}`}
            title={available ? undefined : copy.soon}
            onClick={() => setViewMode(mode)}
          >
            {mode === "canvas" && <Spline size={15} strokeWidth={1.75} aria-hidden="true" />}
            {label}
          </button>
        );
      })}
    </div>
  );
}
