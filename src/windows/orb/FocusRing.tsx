import { timeCopy as t } from "../../copy";
import { clock, useFocus } from "../../hooks/useFocus";
import styles from "./focusRing.module.css";

/** How much wider than the orb the ring is, so it sits around it. */
const GAP = 22;
const STROKE = 3;

/** Progress ring around the orb while a focus session runs. */
export function FocusRing({ orbPx = 52 }: { orbPx?: number }) {
  const { focus, leftMs, progress } = useFocus();
  if (!focus) return null;
  const SIZE = orbPx + GAP;
  const R = (SIZE - STROKE) / 2;
  const C = 2 * Math.PI * R;
  return (
    <span
      className={styles.wrap}
      title={`${t.focusOn(focus.title)} · ${t.focusLeft(clock(leftMs))}`}
    >
      <svg width={SIZE} height={SIZE} viewBox={`0 0 ${SIZE} ${SIZE}`} aria-hidden="true">
        <circle className={styles.track} cx={SIZE / 2} cy={SIZE / 2} r={R} strokeWidth={STROKE} />
        <circle
          className={styles.fill}
          cx={SIZE / 2}
          cy={SIZE / 2}
          r={R}
          strokeWidth={STROKE}
          strokeDasharray={C}
          strokeDashoffset={C * (1 - progress)}
        />
      </svg>
    </span>
  );
}
