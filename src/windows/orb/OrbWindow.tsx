import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState } from "react";
import { onAiState } from "../../api/ai";
import { onVoiceState, panelToggle } from "../../api/desktop";
import { NomaOrb, type OrbState } from "../../components/NomaOrb";
import { copy } from "../../copy";
import styles from "./orb.module.css";

const DRAG_THRESHOLD = 4;

/** Small transparent always-on-top orb: click opens the panel, drag moves it. */
export function OrbWindow() {
  const [state, setState] = useState<OrbState>("idle");
  const down = useRef<{ x: number; y: number } | null>(null);

  useEffect(() => {
    const subs = [
      onVoiceState((s) =>
        setState(
          s.phase === "listening"
            ? "listening"
            : s.phase === "transcribing" || s.phase === "saved"
              ? "processing"
              : s.phase === "error"
                ? "error"
                : "idle",
        ),
      ),
      onAiState((a) =>
        setState((prev) => (prev === "listening" ? prev : a.pending > 0 ? "processing" : "idle")),
      ),
    ];
    return () => subs.forEach((p) => void p.then((un) => un()));
  }, []);

  return (
    <button
      type="button"
      className={styles.root}
      aria-label={copy.orbHere}
      title={copy.orbHint}
      onMouseDown={(e) => {
        if (e.button === 0) down.current = { x: e.screenX, y: e.screenY };
      }}
      onMouseMove={(e) => {
        const d = down.current;
        if (!d) return;
        if (Math.hypot(e.screenX - d.x, e.screenY - d.y) > DRAG_THRESHOLD) {
          down.current = null;
          void getCurrentWindow().startDragging();
        }
      }}
      onMouseUp={() => {
        if (down.current) void panelToggle("panel");
        down.current = null;
      }}
    >
      <NomaOrb size={52} state={state} label={copy.orbHere} className={styles.orb} />
    </button>
  );
}
