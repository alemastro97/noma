import { timeCopy as t } from "../../copy";
import { addDays } from "../calendar/calendar";
import styles from "./Habits.module.css";

/** Streak count and the last 7 days as dots (oldest first, today last). */
export function HabitDots({ streak, last7 }: { streak: number; last7: boolean[] }) {
  const today = new Date();
  return (
    <span className={styles.stats}>
      <span className={styles.streak} data-zero={streak === 0 || undefined}>
        {t.streak(streak)}
      </span>
      <span className={styles.dots}>
        {last7.map((done, i) => {
          const day = addDays(today, i - (last7.length - 1));
          const label = `${day.toLocaleDateString("en-GB", { weekday: "short" })}: ${
            done ? t.dayDone : t.dayMissed
          }`;
          return (
            <span
              key={i}
              className={styles.dot}
              data-done={done || undefined}
              title={label}
              aria-label={label}
              role="img"
            />
          );
        })}
      </span>
    </span>
  );
}
