import { useCallback, useEffect, useState } from "react";
import { onItemsChanged } from "../../api/events";
import { habitsList, type Habit } from "../../api/time";
import { timeCopy as t } from "../../copy";
import { Checkbox } from "../../components/Card/Checkbox";
import { KIND_STYLE } from "../../components/Card/kind";
import { anchorDate, formatDue } from "../../lib/dates";
import { useItemsStore } from "../../stores/items";
import { useUiStore } from "../../stores/ui";
import { useNow } from "../useWorkspace";
import { HabitDots } from "./HabitDots";
import styles from "./Habits.module.css";

/** Habits strip at the top of Today: streak, 7-day dots, tick off today's one. */
export function HabitsSection() {
  const [habits, setHabits] = useState<Habit[]>([]);
  const now = useNow();

  const load = useCallback(() => {
    void habitsList()
      .then((h) => setHabits(h ?? []))
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    load();
    const un = onItemsChanged(load);
    return () => void un.then((f) => f()).catch(() => undefined);
  }, [load]);

  if (habits.length === 0) return null;

  const complete = (id: string) => {
    void useItemsStore.getState().complete(id).then(load);
  };

  return (
    <section className={styles.section} aria-label={t.habits}>
      <h2 className={styles.heading}>
        {t.habits}
        <span className={styles.count}>{habits.length}</span>
      </h2>
      <div className={styles.list}>
        {habits.map(({ item, streak, last7 }) => {
          const at = anchorDate(item);
          return (
            <div key={item.id} className={styles.row}>
              <Checkbox
                checked={false}
                color={KIND_STYLE[item.kind].color}
                size={18}
                onToggle={() => complete(item.id)}
              />
              <button
                type="button"
                className={styles.title}
                onClick={() => useUiStore.getState().openDetail(item.id)}
              >
                {item.title}
              </button>
              {at && <span className={styles.when}>{formatDue(at, now)}</span>}
              <HabitDots streak={streak} last7={last7} />
            </div>
          );
        })}
      </div>
    </section>
  );
}
