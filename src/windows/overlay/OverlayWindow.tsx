import { useCallback, useEffect, useRef, useState } from "react";
import { captureText } from "../../api/ai";
import {
  onVoiceLevel,
  onVoicePartial,
  onVoiceState,
  panelHide,
  panelShow,
  voiceCancel,
  voiceCurrent,
  voiceEdit,
  voiceStop,
} from "../../api/desktop";
import type { VoicePhase, VoiceState } from "../../api/types";
import { NomaOrb, type OrbState } from "../../components/NomaOrb";
import { desktopCopy } from "../../copy";
import { useShortcutLabels } from "../../hooks/useShortcuts";
import glass from "../shared/glass.module.css";
import styles from "./overlay.module.css";
import { bars } from "./summary";

const BAR_COUNT = 44;
const CLOSE_AFTER_MESSAGE = 2400;

type View = VoicePhase;

const ORB_FOR: Record<View, OrbState> = {
  idle: "idle",
  listening: "listening",
  transcribing: "processing",
  editing: "idle",
  saved: "success",
  empty: "idle",
  unavailable: "error",
  error: "error",
};

/** Voice overlay: ⌥Space anywhere → listen → transcript → captured card summary. */
export function OverlayWindow() {
  const keys = useShortcutLabels();
  const [view, setView] = useState<View>("idle");
  const [text, setText] = useState("");
  const [draft, setDraft] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [levels, setLevels] = useState<number[]>([]);
  const closeTimer = useRef<number | undefined>(undefined);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const scheduleClose = useCallback((ms: number) => {
    window.clearTimeout(closeTimer.current);
    closeTimer.current = window.setTimeout(() => {
      void panelHide("overlay");
    }, ms);
  }, []);

  const apply = useCallback(
    (s: VoiceState) => {
      switch (s.phase) {
        case "listening":
          window.clearTimeout(closeTimer.current);
          setMessage(null);
          setText(s.text ?? "");
          setLevels([]);
          setView("listening");
          break;
        case "transcribing":
          setView("transcribing");
          break;
        case "editing":
          setDraft(s.text ?? "");
          setView("editing");
          break;
        case "saved":
          // The backend already closed this window; the card is made in the background.
          setText(s.text ?? "");
          break;
        case "empty":
        case "unavailable":
        case "error":
          setMessage(s.message ?? desktopCopy[s.phase]);
          setView(s.phase);
          scheduleClose(CLOSE_AFTER_MESSAGE);
          break;
        case "idle":
          break;
      }
    },
    [scheduleClose],
  );

  useEffect(() => {
    const subs = [
      onVoiceState(apply),
      onVoiceLevel((l) => setLevels((prev) => [...prev.slice(-BAR_COUNT + 1), l])),
      onVoicePartial((t) => setText(t)),
    ];
    void voiceCurrent()
      .then(apply)
      .catch(() => undefined);
    return () => {
      window.clearTimeout(closeTimer.current);
      subs.forEach((p) => void p.then((un) => un()));
    };
  }, [apply]);

  useEffect(() => {
    if (view === "editing") inputRef.current?.focus();
  }, [view]);

  const saveDraft = useCallback(async () => {
    const value = draft.trim();
    if (!value) {
      void voiceCancel();
      return;
    }
    // Close first; the capture runs in the background.
    void panelHide("overlay");
    try {
      await captureText(value, {});
    } catch (err) {
      setMessage(String(err));
      setView("error");
      scheduleClose(CLOSE_AFTER_MESSAGE);
      void panelShow("overlay");
    }
  }, [draft, scheduleClose]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        window.clearTimeout(closeTimer.current);
        void voiceCancel();
        return;
      }
      if (view === "listening") {
        if (e.key === "Enter") {
          e.preventDefault();
          void voiceStop();
        } else if (e.key === "Tab") {
          e.preventDefault();
          void voiceEdit().then((t) => {
            setDraft(t ?? text);
            setView("editing");
          });
        }
      } else if (view === "editing" && e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        void saveDraft();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [view, text, saveDraft]);

  const status =
    view === "editing"
      ? desktopCopy.editing
      : view === "transcribing"
        ? desktopCopy.transcribing
        : view === "listening" || view === "idle"
          ? desktopCopy.listening
          : null;

  const heights = bars(levels, BAR_COUNT);
  const live = view === "listening";

  return (
    <section className={`${glass.glass} ${styles.panel}`} aria-label="Voice capture">
      <div className={styles.head}>
        <NomaOrb size={64} state={ORB_FOR[view]} />
        <div className={styles.copy}>
          {status && <span className={styles.status}>{status}</span>}
          {view === "editing" ? (
            <textarea
              ref={inputRef}
              className={styles.edit}
              value={draft}
              rows={2}
              aria-label="Edit transcript"
              onChange={(e) => setDraft(e.target.value)}
            />
          ) : message && ["empty", "unavailable", "error"].includes(view) ? (
            <p className={styles.message}>{message}</p>
          ) : (
            <p className={styles.transcript} aria-live="polite">
              {text || <span className={styles.placeholder}>…</span>}
            </p>
          )}
        </div>
      </div>

      {(live || view === "transcribing") && (
        <div className={styles.wave} aria-hidden="true">
          {heights.map((h, i) => (
            <span
              key={i}
              style={{ transform: `scaleY(${live ? h : 0.2})` }}
              className={view === "transcribing" ? styles.idleBar : undefined}
            />
          ))}
        </div>
      )}

      <footer className={styles.foot}>
        {view === "editing" ? (
          <>
            <span className={glass.hint}>
              <span className={glass.kbd}>↵</span>
              {desktopCopy.save}
            </span>
            <span className={glass.hint}>
              <span className={glass.kbd}>Esc</span>
              {desktopCopy.cancel}
            </span>
          </>
        ) : (
          <>
            <span className={glass.hint}>
              <span className={glass.kbd}>↵</span>
              {desktopCopy.save}
            </span>
            <span className={glass.hint}>
              <span className={glass.kbd}>Tab</span>
              {desktopCopy.edit}
            </span>
            <span className={glass.hint}>
              <span className={glass.kbd}>Esc</span>
              {desktopCopy.cancel}
            </span>
          </>
        )}
        <span className={`${glass.hint} ${styles.right}`}>
          {desktopCopy.holdAnywhere} <span className={glass.kbd}>{keys.voice}</span> anywhere
        </span>
      </footer>
    </section>
  );
}
