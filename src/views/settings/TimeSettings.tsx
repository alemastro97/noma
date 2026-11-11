import { timeCopy as t } from "../../copy";
import { useSetting } from "../../hooks/useSetting";
import styles from "./Settings.module.css";

/** Settings for the time features: the evening recap toast. */
export function TimeSettings() {
  const [recap, setRecap] = useSetting("reminders.recap", "1");
  return (
    <section className={styles.section} aria-labelledby="time-title">
      <h2 id="time-title" className={styles.h2}>
        {t.settingsTitle}
      </h2>
      <label className={styles.toggleRow}>
        <span className={styles.toggleText}>
          <span className={styles.toggleLabel}>{t.recap}</span>
          <span className={styles.meta}>{t.recapHint}</span>
        </span>
        <input
          type="checkbox"
          role="switch"
          className={styles.switch}
          checked={recap === "1"}
          disabled={recap === null}
          onChange={(e) => setRecap(e.target.checked ? "1" : "0")}
        />
      </label>
    </section>
  );
}
