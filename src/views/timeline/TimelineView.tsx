import { ChevronLeft, ChevronRight } from "lucide-react";
import { useMemo, useState, type CSSProperties } from "react";
import { timeCopy as t } from "../../copy";
import { KIND_STYLE } from "../../components/Card/kind";
import { anchorDate, formatTime, startOfDay, timeRange, toDate, intlLocale } from "../../lib/dates";
import { addDays, dayKey } from "../calendar/calendar";
import { useCardHandlers, useNow, useWorkspaceData } from "../useWorkspace";
import { layoutBars } from "./layout";
import styles from "./Timeline.module.css";

const DAYS = 14;
/** Days shown before today when the timeline opens. */
const LEAD = 2;
const COL = 188;

/** Horizontal days with cards on their date; events stretch to their end. */
export function TimelineView() {
  const { items, selectedId } = useWorkspaceData();
  const handlers = useCardHandlers();
  const now = useNow();
  const [start, setStart] = useState(() => addDays(startOfDay(new Date()), -LEAD));

  const days = useMemo(() => Array.from({ length: DAYS }, (_, i) => addDays(start, i)), [start]);
  const bars = useMemo(() => layoutBars(items, start, DAYS), [items, start]);
  const lanes = bars.reduce((n, b) => Math.max(n, b.lane + 1), 0);
  const todayKey = dayKey(now);
  const todayIndex = days.findIndex((d) => dayKey(d) === todayKey);
  const grid = { "--col": `${COL}px`, "--days": DAYS } as CSSProperties;

  return (
    <div className={styles.root}>
      <header className={styles.toolbar}>
        <button
          type="button"
          className={styles.iconBtn}
          aria-label={t.previous}
          onClick={() => setStart((s) => addDays(s, -7))}
        >
          <ChevronLeft size={16} strokeWidth={1.75} aria-hidden="true" />
        </button>
        <button
          type="button"
          className={styles.textBtn}
          onClick={() => setStart(addDays(startOfDay(new Date()), -LEAD))}
        >
          {t.today}
        </button>
        <button
          type="button"
          className={styles.iconBtn}
          aria-label={t.next}
          onClick={() => setStart((s) => addDays(s, 7))}
        >
          <ChevronRight size={16} strokeWidth={1.75} aria-hidden="true" />
        </button>
      </header>

      <div className={styles.scroll}>
        <div className={styles.canvas} style={grid} role="grid" aria-label={t.timeline}>
          {todayIndex >= 0 && (
            <div
              className={styles.todayColumn}
              style={{ left: todayIndex * COL }}
              aria-hidden="true"
            />
          )}
          <div className={styles.header} role="row">
            {days.map((d) => (
              <div
                key={dayKey(d)}
                role="columnheader"
                className={styles.dayHead}
                data-today={dayKey(d) === todayKey || undefined}
              >
                <span className={styles.weekday}>
                  {d.toLocaleDateString(intlLocale(), { weekday: "short" })}
                </span>
                <span className={styles.date}>
                  {d.toLocaleDateString(intlLocale(), { day: "numeric", month: "short" })}
                </span>
              </div>
            ))}
          </div>
          <div className={styles.body} style={{ minHeight: Math.max(lanes, 6) * 48 }}>
            {bars.map((bar) => {
              const { item } = bar;
              const at = anchorDate(item)!;
              const { color } = KIND_STYLE[item.kind];
              const timed = at.getHours() !== 0 || at.getMinutes() !== 0;
              // A multi-day span shows its whole range; a single day just the start.
              const multiDay = bar.to > bar.from;
              const meta = multiDay
                ? timeRange(at, toDate(item.endsAt))
                : timed && !bar.clippedStart
                  ? formatTime(at)
                  : null;
              return (
                <button
                  key={item.id}
                  type="button"
                  className={styles.bar}
                  data-kind={item.kind}
                  data-done={item.status === "done" || undefined}
                  data-selected={item.id === selectedId || undefined}
                  data-clip-start={bar.clippedStart || undefined}
                  data-clip-end={bar.clippedEnd || undefined}
                  style={{ gridColumn: `${bar.from + 1} / ${bar.to + 2}`, gridRow: bar.lane + 1 }}
                  title={item.title}
                  onClick={() => handlers.onOpen(item.id)}
                >
                  <span className={styles.dot} style={{ background: color }} aria-hidden="true" />
                  <span className={styles.barTitle}>{item.title}</span>
                  {meta && <span className={styles.time}>{meta}</span>}
                </button>
              );
            })}
            {bars.length === 0 && <p className={styles.nothing}>{t.nothingScheduled}</p>}
          </div>
        </div>
      </div>
    </div>
  );
}
