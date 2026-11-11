import { Bell, Moon, Timer, X } from "lucide-react";
import { useEffect, useState } from "react";
import { reminderDismiss, reminderDone } from "../../api/desktop";
import { focusStart, recapOpen, reminderSnoozeUntil } from "../../api/time";
import type { Item, ReminderToast } from "../../api/types";
import { desktopCopy, timeCopy as t, uiCopy } from "../../copy";
import { FOCUS_MINUTES } from "../../hooks/useFocus";
import { formatDay, formatDue, formatTime, toDate } from "../../lib/dates";
import { PICKER_HOURS, atHour, nextMonday, pickerDays, thisEvening } from "../../lib/snooze";
import glass from "../shared/glass.module.css";
import styles from "./toasts.module.css";
import own from "./time.module.css";

/** "Later…": snooze to this evening, next Monday, or a picked time. */
export function SnoozeLater({ itemId, onResize }: { itemId: string; onResize: () => void }) {
  const [open, setOpen] = useState(false);
  const [day, setDay] = useState(0);
  const [hour, setHour] = useState<number | null>(null);
  useEffect(onResize, [open, onResize]);

  const now = new Date();
  const evening = thisEvening(now);
  const days = pickerDays(now);
  const until = (d: Date) =>
    void reminderSnoozeUntil(itemId, d.toISOString()).catch(() => undefined);
  const picked = hour === null ? null : atHour(days[day]!, hour);
  const pickedOk = picked !== null && picked.getTime() > now.getTime();

  if (!open) {
    return (
      <button type="button" className={own.later} onClick={() => setOpen(true)}>
        {t.later}
      </button>
    );
  }
  return (
    <div className={own.laterPanel}>
      <div className={styles.actions}>
        {evening && (
          <button
            type="button"
            className={`${glass.btn} ${styles.slim}`}
            onClick={() => until(evening)}
          >
            {t.thisEvening}
          </button>
        )}
        <button
          type="button"
          className={`${glass.btn} ${styles.slim}`}
          onClick={() => until(nextMonday(now))}
        >
          {t.nextMonday}
        </button>
      </div>
      <span className={own.groupLabel}>{t.pickTime}</span>
      <div className={own.pick} role="radiogroup" aria-label={t.pickTime}>
        {days.map((d, i) => (
          <button
            key={i}
            type="button"
            role="radio"
            aria-checked={day === i}
            className={own.pickChip}
            onClick={() => setDay(i)}
          >
            {formatDay(d, now)}
          </button>
        ))}
      </div>
      <div className={own.pick} role="radiogroup" aria-label={t.pickTime}>
        {PICKER_HOURS.map((h) => {
          const past = atHour(days[day]!, h).getTime() <= now.getTime();
          return (
            <button
              key={h}
              type="button"
              role="radio"
              aria-checked={hour === h}
              disabled={past}
              className={own.pickChip}
              onClick={() => setHour(h)}
            >
              {formatTime(atHour(days[day]!, h))}
            </button>
          );
        })}
      </div>
      <div className={styles.actions}>
        <button
          type="button"
          className={`${glass.btn} ${styles.slim}`}
          disabled={!pickedOk}
          onClick={() => picked && until(picked)}
        >
          {t.snooze}
        </button>
      </div>
    </div>
  );
}

function Close({ id }: { id: string }) {
  return (
    <button
      type="button"
      className={styles.close}
      aria-label={desktopCopy.dismiss}
      onClick={() => void reminderDismiss(id)}
    >
      <X size={14} />
    </button>
  );
}

/** "Due in 1 hour": a reminder set before the card's due time. */
export function BeforeToast({
  toast,
  item,
  now,
  onResize,
}: {
  toast: ReminderToast;
  item: Item;
  now: Date;
  onResize: () => void;
}) {
  const due = toDate(item.dueAt);
  return (
    <section className={`${glass.glass} ${styles.toast}`} aria-label={uiCopy.regions.reminder}>
      <div className={styles.icon} aria-hidden="true">
        <Bell size={20} strokeWidth={1.75} />
      </div>
      <div className={styles.body}>
        <div className={styles.text}>
          <span className={styles.title}>{item.title}</span>
          {due && <span className={styles.when}>{t.due(formatDue(due, now))}</span>}
        </div>
        <div className={styles.actions}>
          <button
            type="button"
            className={`${glass.btn} ${glass.btnDone}`}
            onClick={() => void reminderDone(item.id)}
          >
            {desktopCopy.done}
          </button>
        </div>
        <SnoozeLater itemId={item.id} onResize={onResize} />
      </div>
      <Close id={toast.id} />
    </section>
  );
}

/** End of a focus session: mark the card done or go another round. */
export function FocusDoneToast({ toast, item }: { toast: ReminderToast; item: Item }) {
  return (
    <section className={`${glass.glass} ${styles.toast}`} aria-label={t.focusDone}>
      <div className={`${styles.icon} ${own.focusIcon}`} aria-hidden="true">
        <Timer size={20} strokeWidth={1.75} />
      </div>
      <div className={styles.body}>
        <div className={styles.text}>
          <span className={styles.missedTitle}>{t.focusDone}</span>
          <span className={styles.place}>{item.title}</span>
        </div>
        <div className={styles.actions}>
          <button
            type="button"
            className={glass.btn}
            onClick={() => void focusStart(item.id, FOCUS_MINUTES).catch(() => undefined)}
          >
            {t.focusAgain(FOCUS_MINUTES)}
          </button>
          {item.status !== "done" && (
            <button
              type="button"
              className={`${glass.btn} ${glass.btnDone}`}
              onClick={() => void reminderDone(item.id)}
            >
              {t.markDone}
            </button>
          )}
        </div>
      </div>
      <Close id={toast.id} />
    </section>
  );
}

const RECAP_ROWS = 3;

/** Evening recap: what got done today and what is still open. */
export function RecapToast({ toast }: { toast: ReminderToast }) {
  const done = toast.items.filter((i) => i.status === "done");
  const open = toast.items.filter((i) => i.status !== "done");
  const list = (items: Item[]) => (
    <ul className={styles.missed}>
      {items.slice(0, RECAP_ROWS).map((i) => (
        <li key={i.id}>{i.title}</li>
      ))}
      {items.length > RECAP_ROWS && <li>{desktopCopy.andMore(items.length - RECAP_ROWS)}</li>}
    </ul>
  );
  return (
    <section className={`${glass.glass} ${styles.toast}`} aria-label={t.recap}>
      <div className={`${styles.icon} ${own.recapIcon}`} aria-hidden="true">
        <Moon size={20} strokeWidth={1.75} />
      </div>
      <div className={styles.body}>
        <span className={styles.missedTitle}>{t.recapTitle(done.length, open.length)}</span>
        {open.length > 0 && (
          <div className={own.group}>
            <span className={own.groupLabel}>{t.recapOpen}</span>
            {list(open)}
          </div>
        )}
        {done.length > 0 && (
          <div className={own.group}>
            <span className={own.groupLabel}>{t.recapDone}</span>
            {list(done)}
          </div>
        )}
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
            onClick={() => void recapOpen()}
          >
            {desktopCopy.seeToday}
          </button>
        </div>
      </div>
    </section>
  );
}
