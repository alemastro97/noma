import { Timer } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { onItemsChanged } from "../../api/events";
import {
  habitSet,
  onFocusChanged,
  reminderOffsetsSet,
  timeItemInfo,
  type ItemTime,
} from "../../api/time";
import type { Item } from "../../api/types";
import { timeCopy as t } from "../../copy";
import { FOCUS_MINUTES, clock, useFocus } from "../../hooks/useFocus";
import { toastError } from "../../stores/toasts";
import { HabitDots } from "../../views/habits/HabitDots";
import styles from "./DetailPanel.module.css";
import own from "./TimeSection.module.css";

/** "Remind me before" presets, in minutes. */
export const OFFSET_PRESETS = [10, 60, 1440, 10080];

/** Detail-panel block for the time features: reminders before, habit, focus timer. */
export function TimeSection({ item }: { item: Item }) {
  const [info, setInfo] = useState<ItemTime | null>(null);
  const { focus, leftMs, start, stop } = useFocus();

  const load = useCallback(() => {
    void timeItemInfo(item.id)
      .then((i) => i && setInfo(i))
      .catch(() => undefined);
  }, [item.id]);

  useEffect(() => {
    load();
    const subs = [onItemsChanged(load), onFocusChanged(load)];
    return () => subs.forEach((p) => void p.then((f) => f()).catch(() => undefined));
  }, [load, item.updatedAt]);

  const offsets = info?.offsets ?? [];
  const toggleOffset = (m: number) => {
    const next = offsets.includes(m) ? offsets.filter((x) => x !== m) : [...offsets, m];
    setInfo((i) => (i ? { ...i, offsets: next } : i));
    void reminderOffsetsSet(item.id, next)
      .then(load)
      .catch((e) => {
        toastError(String(e));
        load();
      });
  };

  const toggleHabit = (on: boolean) => {
    setInfo((i) => (i ? { ...i, habit: on } : i));
    void habitSet(item.id, on)
      .then(load)
      .catch((e) => {
        toastError(String(e));
        load();
      });
  };

  const focusing = focus?.itemId === item.id;
  const done = item.status === "done";

  return (
    <>
      <div className={styles.field}>
        <span className={styles.label}>{t.remindBefore}</span>
        <div
          className={own.chips}
          role="group"
          aria-label={t.remindBefore}
          title={item.dueAt ? undefined : t.remindBeforeNeedsDue}
        >
          {OFFSET_PRESETS.map((m) => (
            <button
              key={m}
              type="button"
              className={own.chip}
              aria-pressed={offsets.includes(m)}
              disabled={!item.dueAt || done}
              onClick={() => toggleOffset(m)}
            >
              {t.offsetLabel(m)}
            </button>
          ))}
        </div>
      </div>

      <div className={styles.field}>
        <label className={own.toggleRow} title={item.recurrence ? undefined : t.habitNeedsRepeat}>
          <span className={own.toggleText}>
            <span className={own.toggleLabel}>{t.habit}</span>
            <span className={styles.label}>
              {item.recurrence ? t.habitHint : t.habitNeedsRepeat}
            </span>
          </span>
          <input
            type="checkbox"
            role="switch"
            className={own.switch}
            checked={!!info?.habit}
            disabled={!info || (!item.recurrence && !info.habit)}
            onChange={(e) => toggleHabit(e.target.checked)}
          />
        </label>
        {info?.habit && info.habitStats && (
          <HabitDots streak={info.habitStats.streak} last7={info.habitStats.last7} />
        )}
      </div>

      <div className={styles.field}>
        <span className={styles.label}>{t.focus}</span>
        <div className={own.focusRow}>
          {focusing ? (
            <>
              <span className={own.focusing}>
                <Timer size={14} strokeWidth={1.75} aria-hidden="true" />
                {t.focusing} · {t.focusLeft(clock(leftMs))}
              </span>
              <button type="button" className={own.chip} onClick={stop}>
                {t.stopFocus}
              </button>
            </>
          ) : (
            <button
              type="button"
              className={own.chip}
              disabled={done}
              onClick={() => start(item.id)}
            >
              <Timer size={14} strokeWidth={1.75} aria-hidden="true" />
              {t.startFocus(FOCUS_MINUTES)}
            </button>
          )}
          {info && (
            <span className={own.stats}>
              {t.focusStats(info.focus.sessions, info.focus.minutes)}
            </span>
          )}
        </div>
      </div>
    </>
  );
}
