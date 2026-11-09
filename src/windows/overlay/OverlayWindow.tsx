import { useCallback, useEffect, useRef, useState } from "react";
import { onCapture, captureText } from "../../api/ai";
import {
  onVoiceLevel,
  onVoicePartial,
  onVoiceState,
  panelHide,
  voiceCancel,
  voiceCurrent,
  voiceEdit,
  voiceStop,
} from "../../api/desktop";
import { getItem } from "../../api/items";
import { listProjects } from "../../api/projects";
import type { VoicePhase, VoiceState } from "../../api/types";
import { NomaOrb, type OrbState } from "../../components/NomaOrb";
import { desktopCopy } from "../../copy";
import glass from "../shared/glass.module.css";
import styles from "./overlay.module.css";
import { bars, summarize, type SummaryChip } from "./summary";

const BAR_COUNT = 44;
const CLOSE_AFTER_RESULT = 1800;
const CLOSE_AFTER_MESSAGE = 2400;
const RESULT_TIMEOUT = 8000;

type View = VoicePhase | "thinking" | "result";

const ORB_FOR: Record<View, OrbState> = {
  idle: "idle",
  listening: "listening",
  transcribing: "processing",
  editing: "idle",
  saved: "processing",
  thinking: "processing",
  result: "success",
  empty: "idle",
  unavailable: "error",
  error: "error",
};

/** Voice overlay: ⌥Space anywhere → listen → transcript → captured card summary. */
export function OverlayWindow() {
  const [view, setView] = useState<View>("idle");
  const [text, setText] = useState("");
  const [draft, setDraft] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [chips, setChips] = useState<SummaryChip[]>([]);
  const [levels, setLevels] = useState<number[]>([]);
  const pendingCapture = useRef<string | null>(null);
  const closeTimer = useRef<number | undefined>(undefined);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const scheduleClose = useCallback((ms: number) => {
    window.clearTimeout(closeTimer.current);
    closeTimer.current = window.setTimeout(() => {
      void panelHide("overlay");
    }, ms);
  }, []);

  const showResult = useCallback(
    async (itemId: string | null) => {
      pendingCapture.current = null;
      const item = itemId ? await getItem(itemId).catch(() => null) : null;
      if (item) {
        const projects = await listProjects().catch(() => []);
        const project = projects.find((p) => p.id === item.projectId);
        setText(item.title);
        setChips(summarize(item, project, new Date()));
      } else {
        setChips([]);
        setMessage(desktopCopy.captureSaved);
      }
      setView("result");
      scheduleClose(CLOSE_AFTER_RESULT);
    },
    [scheduleClose],
  );

  const awaitCapture = useCallback(
    (captureId: string | null, itemId: string | null) => {
      pendingCapture.current = captureId;
      setView("thinking");
      window.clearTimeout(closeTimer.current);
      // The AI may be offline; the card is already saved, so don't hang around.
      closeTimer.current = window.setTimeout(() => {
        if (pendingCapture.current === captureId) void showResult(itemId);
      }, RESULT_TIMEOUT);
    },
    [showResult],
  );

  const apply = useCallback(
    (s: VoiceState) => {
      switch (s.phase) {
        case "listening":
          window.clearTimeout(closeTimer.current);
          pendingCapture.current = null;
          setChips([]);
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
          setText(s.text ?? "");
          awaitCapture(s.captureId, s.itemId);
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
    [awaitCapture, scheduleClose],
  );

  useEffect(() => {
    const subs = [
      onVoiceState(apply),
      onVoiceLevel((l) => setLevels((prev) => [...prev.slice(-BAR_COUNT + 1), l])),
      onVoicePartial((t) => setText(t)),
      onCapture((e) => {
        if (e.captureId !== pendingCapture.current) return;
        window.clearTimeout(closeTimer.current);
        void showResult(e.itemIds[0] ?? null);
      }),
    ];
    void voiceCurrent()
      .then(apply)
      .catch(() => undefined);
    return () => {
      window.clearTimeout(closeTimer.current);
      subs.forEach((p) => void p.then((un) => un()));
    };
  }, [apply, showResult]);

  useEffect(() => {
    if (view === "editing") inputRef.current?.focus();
  }, [view]);

  const saveDraft = useCallback(async () => {
    const value = draft.trim();
    if (!value) {
      void voiceCancel();
      return;
    }
    setText(value);
    try {
      const res = await captureText(value, {});
      awaitCapture(res.captureId, res.item.id);
    } catch (err) {
      setMessage(String(err));
      setView("error");
      scheduleClose(CLOSE_AFTER_MESSAGE);
    }
  }, [draft, awaitCapture, scheduleClose]);

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
      } else if (view === "result" && e.key === "Enter") {
        void panelHide("overlay");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [view, text, saveDraft]);

  const status =
    view === "result"
      ? desktopCopy.saved
      : view === "thinking" || view === "saved"
        ? desktopCopy.thinking
        : view === "editing"
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
          {status && (
            <span className={`${styles.status} ${view === "result" ? styles.ok : ""}`}>
              {status}
            </span>
          )}
          {view === "editing" ? (
            <textarea
              ref={inputRef}
              className={styles.edit}
              value={draft}
              rows={2}
              aria-label="Edit transcript"
              onChange={(e) => setDraft(e.target.value)}
            />
          ) : message && !["result", "thinking"].includes(view) ? (
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

      {chips.length > 0 && (
        <div className={styles.chips}>
          {chips.map((c) => (
            <span
              key={c.key}
              className={`${glass.chip} ${c.tone === "reminder" ? styles.reminder : ""}`}
            >
              {c.dot && <span className={glass.dot} style={{ background: c.dot }} />}
              {c.label}
            </span>
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
          {desktopCopy.holdAnywhere} <span className={glass.kbd}>⌥ Space</span> anywhere
        </span>
      </footer>
    </section>
  );
}
