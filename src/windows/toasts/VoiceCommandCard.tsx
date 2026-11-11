import { Check, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { filedOpen, reminderDismiss } from "../../api/desktop";
import type { ReminderToast } from "../../api/types";
import { voiceCommandUndo, voiceToastKind } from "../../api/voice";
import { desktopCopy, voiceCopy } from "../../copy";
import { anchorDate, formatDue } from "../../lib/dates";
import glass from "../shared/glass.module.css";
import styles from "./toasts.module.css";

const CLOSE_AFTER = 7000;

/** Confirms a voice command ("mark … done", "move … to tomorrow"): Undo or Open. */
export function VoiceCommandCard({ toast }: { toast: ReminderToast }) {
  const kind = voiceToastKind(toast.id);
  const item = toast.items[0];
  const [hover, setHover] = useState(false);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => {
    window.clearTimeout(timer.current);
    if (!hover) {
      timer.current = window.setTimeout(() => void reminderDismiss(toast.id), CLOSE_AFTER);
    }
    return () => window.clearTimeout(timer.current);
  }, [hover, toast.id]);

  if (!item || !kind) return null;
  const when = anchorDate(item);
  const line =
    kind === "done"
      ? voiceCopy.markedDone
      : when
        ? voiceCopy.movedTo(formatDue(when, new Date()))
        : voiceCopy.moved;
  // Completing a repeating card advances its series: not undoable from here.
  const canUndo = !(kind === "done" && item.recurrence);

  return (
    <section
      className={`${glass.glass} ${styles.toast}`}
      aria-label={line}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
    >
      <div className={`${styles.icon} ${styles.filedIcon}`} aria-hidden="true">
        <Check size={20} strokeWidth={1.75} />
      </div>
      <div className={styles.body}>
        <div className={styles.text}>
          <span className={styles.title}>{item.title}</span>
          <span className={styles.place} aria-live="polite">
            {line}
          </span>
        </div>
        <div className={styles.actions}>
          {canUndo && (
            <button
              type="button"
              className={glass.btn}
              onClick={() => void voiceCommandUndo(toast.id)}
            >
              {desktopCopy.undo}
            </button>
          )}
          <button
            type="button"
            className={`${glass.btn} ${glass.btnDone}`}
            onClick={() => void filedOpen(toast.id, item.id)}
          >
            {desktopCopy.open}
          </button>
        </div>
      </div>
      <button
        type="button"
        className={styles.close}
        aria-label={desktopCopy.dismiss}
        onClick={() => void reminderDismiss(toast.id)}
      >
        <X size={14} />
      </button>
    </section>
  );
}
