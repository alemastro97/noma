import { Timer } from "lucide-react";
import { timeCopy as t } from "../../copy";
import { clock, type FocusClock } from "../../hooks/useFocus";
import styles from "./focus.module.css";

/** Class for a panel row that carries a `FocusButton`. */
export const focusRowClass = styles.item;

/** The running focus session at the top of the orb panel. */
export function FocusBar({ clock: c }: { clock: FocusClock }) {
  if (!c.focus) return null;
  return (
    <div className={styles.bar}>
      <Timer size={14} strokeWidth={1.75} aria-hidden="true" />
      <span className={styles.title}>{c.focus.title}</span>
      <span className={styles.left}>{clock(c.leftMs)}</span>
      <button type="button" className={styles.stop} onClick={c.stop}>
        {t.stopFocus}
      </button>
      <span className={styles.track} aria-hidden="true">
        <span className={styles.fill} style={{ width: `${c.progress * 100}%` }} />
      </span>
    </div>
  );
}

/** Small timer button beside a row: start a focus session on that card. */
export function FocusButton({ title, onStart }: { title: string; onStart: () => void }) {
  return (
    <button
      type="button"
      className={styles.start}
      aria-label={t.focusOn(title)}
      title={t.focusOn(title)}
      onClick={onStart}
    >
      <Timer size={13} strokeWidth={1.75} aria-hidden="true" />
    </button>
  );
}
