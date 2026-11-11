import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { Bell, X } from "lucide-react";
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  briefOpen,
  onRemindersChanged,
  reminderDismiss,
  reminderDone,
  reminderSnooze,
  remindersActive,
} from "../../api/desktop";
import type { Item, ReminderToast } from "../../api/types";
import { desktopCopy } from "../../copy";
import glass from "../shared/glass.module.css";
import { FiledCard } from "./FiledCard";
import styles from "./toasts.module.css";
import { whenLabel } from "./when";

const WIDTH = 420;

/** Reminder toasts, stacked top-right over whatever the user is doing. */
export function ToastsWindow() {
  const [toasts, setToasts] = useState<ReminderToast[]>([]);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void remindersActive()
      .then(setToasts)
      .catch(() => undefined);
    const un = onRemindersChanged(setToasts);
    return () => void un.then((f) => f());
  }, []);

  // A card that grows (the Move menu) asks for a new measure.
  const [measure, setMeasure] = useState(0);
  const remeasure = useCallback(() => setMeasure((n) => n + 1), []);

  // The window hugs its content so the transparent area never blocks clicks.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const h = Math.max(1, Math.ceil(el.getBoundingClientRect().height));
    try {
      void getCurrentWindow()
        .setSize(new LogicalSize(WIDTH, h))
        .catch(() => undefined);
    } catch {
      // Plain browser (dev mock): no window to size.
    }
  }, [toasts, measure]);

  const now = new Date();

  return (
    <div ref={ref} className={styles.stack}>
      {toasts.map((t) => {
        const first = t.items[0];
        if (t.filed) return <FiledCard key={t.id} toast={t} onResize={remeasure} />;
        if (t.brief) return <BriefToast key={t.id} toast={t} />;
        if (t.missed) return <MissedToast key={t.id} toast={t} />;
        return first ? <ReminderCard key={t.id} toastId={t.id} item={first} now={now} /> : null;
      })}
    </div>
  );
}

function ReminderCard({ toastId, item, now }: { toastId: string; item: Item; now: Date }) {
  return (
    <section className={`${glass.glass} ${styles.toast}`} aria-label="Reminder">
      <Icon />
      <div className={styles.body}>
        <div className={styles.text}>
          <span className={styles.title}>{item.title}</span>
          <span className={styles.when}>{whenLabel(item, now)}</span>
        </div>
        <div className={styles.actions}>
          <button
            type="button"
            className={glass.btn}
            onClick={() => void reminderSnooze(item.id, { minutes: 60 })}
          >
            {desktopCopy.snooze1h}
          </button>
          <button
            type="button"
            className={`${glass.btn} ${styles.slim}`}
            onClick={() => void reminderSnooze(item.id, { tomorrow: true })}
          >
            {desktopCopy.tomorrow}
          </button>
          <button
            type="button"
            className={`${glass.btn} ${glass.btnDone}`}
            onClick={() => void reminderDone(item.id)}
          >
            {desktopCopy.done}
          </button>
        </div>
      </div>
      <button
        type="button"
        className={styles.close}
        aria-label={desktopCopy.dismiss}
        onClick={() => void reminderDismiss(toastId)}
      >
        <X size={14} />
      </button>
    </section>
  );
}

function MissedToast({ toast }: { toast: ReminderToast }) {
  return (
    <section className={`${glass.glass} ${styles.toast}`} aria-label="Missed reminders">
      <Icon />
      <div className={styles.body}>
        <span className={styles.missedTitle}>{desktopCopy.missed(toast.items.length)}</span>
        <ul className={styles.missed}>
          {toast.items.slice(0, 4).map((i) => (
            <li key={i.id}>{i.title}</li>
          ))}
        </ul>
        <div className={styles.actions}>
          <button
            type="button"
            className={glass.btn}
            onClick={() => void reminderDismiss(toast.id)}
          >
            {desktopCopy.dismiss}
          </button>
        </div>
      </div>
    </section>
  );
}

function BriefToast({ toast }: { toast: ReminderToast }) {
  const more = toast.items.length - 4;
  return (
    <section className={`${glass.glass} ${styles.toast}`} aria-label="Today">
      <Icon />
      <div className={styles.body}>
        <span className={styles.missedTitle}>{desktopCopy.briefTitle(toast.items.length)}</span>
        <ul className={styles.missed}>
          {toast.items.slice(0, 4).map((i) => (
            <li key={i.id}>{i.title}</li>
          ))}
          {more > 0 && <li>{desktopCopy.andMore(more)}</li>}
        </ul>
        <div className={styles.actions}>
          <button
            type="button"
            className={glass.btn}
            onClick={() => void reminderDismiss(toast.id)}
          >
            {desktopCopy.dismiss}
          </button>
          <button
            type="button"
            className={`${glass.btn} ${glass.btnDone}`}
            onClick={() => void briefOpen()}
          >
            {desktopCopy.seeToday}
          </button>
        </div>
      </div>
    </section>
  );
}

function Icon() {
  return (
    <div className={styles.icon} aria-hidden="true">
      <Bell size={20} strokeWidth={1.75} />
    </div>
  );
}
