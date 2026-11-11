import { ChevronLeft, ChevronRight } from "lucide-react";
import { memo, useMemo, useState, type DragEvent } from "react";
import type { Item } from "../../api/types";
import { timeCopy as t } from "../../copy";
import { KIND_STYLE } from "../../components/Card/kind";
import { anchorDate, formatTime, longDate } from "../../lib/dates";
import { useItemsStore } from "../../stores/items";
import { useUiStore } from "../../stores/ui";
import { useCardHandlers, useNow, useWorkspaceData, type CardHandlers } from "../useWorkspace";
import {
  dayKey,
  itemsByDay,
  quickAddDue,
  reschedulePatch,
  step,
  visibleDays,
  type CalendarMode,
} from "./calendar";
import styles from "./Calendar.module.css";

const DRAG_TYPE = "application/x-noma-item";
/** Chips per day before "+N more" in the month grid. */
const MONTH_CHIPS = 3;

const Chip = memo(function Chip({
  item,
  selected,
  handlers,
}: {
  item: Item;
  selected: boolean;
  handlers: CardHandlers;
}) {
  const { color } = KIND_STYLE[item.kind];
  const at = anchorDate(item);
  const timed = at && (at.getHours() !== 0 || at.getMinutes() !== 0);
  return (
    <button
      type="button"
      className={styles.chip}
      draggable
      data-done={item.status === "done" || undefined}
      data-selected={selected || undefined}
      onDragStart={(e) => {
        e.dataTransfer.setData(DRAG_TYPE, item.id);
        e.dataTransfer.effectAllowed = "move";
      }}
      onClick={(e) => {
        e.stopPropagation();
        handlers.onSelect(item.id);
        handlers.onOpen(item.id);
      }}
      title={item.title}
    >
      <span className={styles.dot} style={{ background: color }} aria-hidden="true" />
      {timed && <span className={styles.time}>{formatTime(at)}</span>}
      <span className={styles.chipTitle}>{item.title}</span>
    </button>
  );
});

function title(anchor: Date, mode: CalendarMode, days: Date[]): string {
  if (mode === "month") {
    return anchor.toLocaleDateString("en-GB", { month: "long", year: "numeric" });
  }
  const first = days[0]!;
  const last = days[days.length - 1]!;
  const fmt = (d: Date) => d.toLocaleDateString("en-GB", { day: "numeric", month: "short" });
  return `${fmt(first)} – ${fmt(last)} ${last.getFullYear()}`;
}

/** Month and week grids: cards sit on their due (or reminder) day; drag to reschedule. */
export function CalendarView() {
  const { items, selectedId } = useWorkspaceData();
  const handlers = useCardHandlers();
  const now = useNow();
  const [mode, setMode] = useState<CalendarMode>("month");
  const [anchor, setAnchor] = useState(() => new Date());
  const [over, setOver] = useState<string | null>(null);

  const days = useMemo(() => visibleDays(anchor, mode), [anchor, mode]);
  const byDay = useMemo(() => itemsByDay(items), [items]);
  const todayKey = dayKey(now);

  const onDrop = (day: Date) => (e: DragEvent) => {
    e.preventDefault();
    setOver(null);
    const id = e.dataTransfer.getData(DRAG_TYPE);
    const item = id ? useItemsStore.getState().items[id] : undefined;
    const patch = item ? reschedulePatch(item, day) : null;
    if (item && patch) void useItemsStore.getState().update(item.id, patch);
  };

  return (
    <div className={styles.root}>
      <header className={styles.toolbar}>
        <h2 className={styles.title}>{title(anchor, mode, days)}</h2>
        <div className={styles.nav}>
          <button
            type="button"
            className={styles.iconBtn}
            aria-label={t.previous}
            onClick={() => setAnchor((a) => step(a, mode, -1))}
          >
            <ChevronLeft size={16} strokeWidth={1.75} aria-hidden="true" />
          </button>
          <button type="button" className={styles.textBtn} onClick={() => setAnchor(new Date())}>
            {t.today}
          </button>
          <button
            type="button"
            className={styles.iconBtn}
            aria-label={t.next}
            onClick={() => setAnchor((a) => step(a, mode, 1))}
          >
            <ChevronRight size={16} strokeWidth={1.75} aria-hidden="true" />
          </button>
        </div>
        <div className={styles.modes} role="radiogroup" aria-label={t.calendar}>
          {(["month", "week"] as const).map((m) => (
            <button
              key={m}
              type="button"
              role="radio"
              aria-checked={mode === m}
              className={styles.mode}
              onClick={() => setMode(m)}
            >
              {m === "month" ? t.month : t.week}
            </button>
          ))}
        </div>
      </header>

      <div className={styles.weekdays} aria-hidden="true">
        {t.weekdays.map((w) => (
          <span key={w}>{w}</span>
        ))}
      </div>
      <div className={styles.grid} data-mode={mode} role="grid" aria-label={t.calendar}>
        {days.map((day) => {
          const key = dayKey(day);
          const list = byDay.get(key) ?? [];
          const shown = mode === "month" ? list.slice(0, MONTH_CHIPS) : list;
          const hidden = list.length - shown.length;
          return (
            <div
              key={key}
              role="gridcell"
              className={styles.day}
              data-today={key === todayKey || undefined}
              data-outside={(mode === "month" && day.getMonth() !== anchor.getMonth()) || undefined}
              data-over={over === key || undefined}
              aria-label={longDate(day)}
              title={t.addOn(longDate(day))}
              onClick={() =>
                useUiStore.getState().openQuickAdd({ kind: "task", dueAt: quickAddDue(day) })
              }
              onDragOver={(e) => {
                if (!e.dataTransfer.types.includes(DRAG_TYPE)) return;
                e.preventDefault();
                e.dataTransfer.dropEffect = "move";
                if (over !== key) setOver(key);
              }}
              onDragLeave={(e) => {
                if (!e.currentTarget.contains(e.relatedTarget as Node)) setOver(null);
              }}
              onDrop={onDrop(day)}
            >
              <span className={styles.dayNumber}>{day.getDate()}</span>
              <div className={styles.chips}>
                {shown.map((item) => (
                  <Chip
                    key={item.id}
                    item={item}
                    selected={item.id === selectedId}
                    handlers={handlers}
                  />
                ))}
                {hidden > 0 && (
                  <button
                    type="button"
                    className={styles.more}
                    onClick={(e) => {
                      e.stopPropagation();
                      setAnchor(day);
                      setMode("week");
                    }}
                  >
                    {t.more(hidden)}
                  </button>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
