import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState } from "react";
import { onAiState } from "../../api/ai";
import { onVoiceState, panelToggle, voiceStop } from "../../api/desktop";
import { NomaOrb, type OrbState } from "../../components/NomaOrb";
import { copy } from "../../copy";
import { useShortcutLabels } from "../../hooks/useShortcuts";
import styles from "./orb.module.css";

const DRAG_THRESHOLD = 4;
const FLASH_MS = 1400;

/** Small transparent always-on-top orb: pulses while listening, click opens the panel, drag moves it. */
export function OrbWindow() {
  const keys = useShortcutLabels();
  const [state, setState] = useState<OrbState>("idle");
  const listening = useRef(false);
  const flash = useRef<number | undefined>(undefined);
  const down = useRef<{ x: number; y: number } | null>(null);

  useEffect(() => {
    const show = (next: OrbState, ms?: number) => {
      window.clearTimeout(flash.current);
      setState(next);
      if (ms) flash.current = window.setTimeout(() => setState("idle"), ms);
    };
    const subs = [
      onVoiceState((s) => {
        listening.current = s.phase === "listening";
        if (s.phase === "listening") show("listening");
        // Transcribing and AI work stay invisible: back to rest at once.
        else if (s.phase === "saved") show("success", FLASH_MS);
        else if (s.phase === "error" || s.phase === "unavailable") show("error", 2400);
        else show("idle");
      }),
      onAiState((a) => {
        if (!listening.current && a.phase === "error") show("error", 2400);
      }),
    ];
    return () => {
      window.clearTimeout(flash.current);
      subs.forEach((p) => void p.then((un) => un()));
    };
  }, []);

  return (
    <button
      type="button"
      className={styles.root}
      aria-label={copy.orbHere}
      title={copy.orbHint(keys.voice)}
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
        // While listening a click means "I'm done"; otherwise it opens the panel.
        if (down.current) void (listening.current ? voiceStop() : panelToggle("panel"));
        down.current = null;
      }}
    >
      <NomaOrb size={52} state={state} label={copy.orbHere} className={styles.orb} />
    </button>
  );
}
