import { Bell, Flag, Pin, Repeat } from "lucide-react";
import { memo, type KeyboardEvent, type ReactNode } from "react";
import type { Item, Project } from "../../api/types";
import { cardsCopy, priorityLabel, uiCopy, cardCopy } from "../../copy";
import { Checkbox } from "../../components/Card/Checkbox";
import { KIND_STYLE } from "../../components/Card/kind";
import { anchorDate, formatDue, formatTime, timeRange, toDate } from "../../lib/dates";
import { describeRule } from "../../lib/recurrence";
import { useCardHandlers, useNow, useWorkspaceData, type CardHandlers } from "../useWorkspace";
import { groupItems, type GroupMode } from "./group";
import styles from "./List.module.css";
import { startCardDrag } from "../../components/Sidebar/dropTarget";

interface RowProps {
  item: Item;
  project?: Project;
  mode: GroupMode;
  now: Date;
  selected: boolean;
  fresh: boolean;
  handlers: CardHandlers;
}

const Row = memo(function Row({ item, project, mode, now, selected, fresh, handlers }: RowProps) {
  const done = item.status === "done";
  const anchor = anchorDate(item);
  const { color, Icon } = KIND_STYLE[item.kind];
  const isReminder = item.kind === "reminder";
  const isEvent = item.kind === "event";
  const meta = anchor
    ? isEvent
      ? timeRange(anchor, toDate(item.endsAt))
      : mode === "today" && anchor.toDateString() === now.toDateString()
        ? formatTime(anchor)
        : formatDue(anchor, now)
    : null;

  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter") handlers.onOpen(item.id);
    else if (e.key === " ") {
      e.preventDefault();
      handlers.onToggle(item.id);
    }
  };

  return (
    <div
      role="row"
      tabIndex={0}
      className={styles.row}
      data-selected={selected || undefined}
      data-done={done || undefined}
      data-fresh={fresh || undefined}
      data-capturing={item.capturing || undefined}
      aria-label={item.title}
      data-item-id={item.id}
      draggable
      onDragStart={(e) => startCardDrag(e, item.id)}
      onClick={() => handlers.onSelect(item.id)}
      onDoubleClick={() => handlers.onOpen(item.id)}
      onKeyDown={onKeyDown}
    >
      {item.kind === "task" || done ? (
        <Checkbox
          checked={done}
          color={color}
          size={18}
          onToggle={() => handlers.onToggle(item.id)}
        />
      ) : (
        <button
          type="button"
          className={styles.kindBox}
          style={{ borderColor: color, color }}
          aria-label={cardCopy.complete}
          onClick={(e) => {
            e.stopPropagation();
            handlers.onToggle(item.id);
          }}
        >
          <Icon size={11} strokeWidth={2.4} aria-hidden={true} />
        </button>
      )}
      <span className={styles.title}>{item.title}</span>
      {item.pinned && (
        <span className={styles.repeat} title={cardsCopy.pinned}>
          <Pin size={13} strokeWidth={1.75} aria-label={cardsCopy.pinned} />
        </span>
      )}
      {item.capturing && <span className={styles.reading}>Reading…</span>}
      {item.hint && !item.capturing && (
        <span className={styles.hintDot} title={uiCopy.needsCheck} />
      )}
      {item.recurrence && (
        <span className={styles.repeat} title={describeRule(item.recurrence)}>
          <Repeat size={13} strokeWidth={1.75} aria-hidden="true" />
        </span>
      )}
      {(item.priority === "high" || item.priority === "urgent") && !done && (
        <span className={styles.priority}>
          <Flag size={12} strokeWidth={2} aria-hidden="true" />
          {priorityLabel[item.priority]}
        </span>
      )}
      {project && <span className={styles.tag}>{project.name}</span>}
      {meta && (
        <span
          className={styles.meta}
          data-kind={isReminder ? "reminder" : isEvent ? "event" : undefined}
          data-overdue={
            !done && anchor && anchor.getTime() < now.getTime() && !isEvent ? true : undefined
          }
        >
          {isReminder && <Bell size={12} strokeWidth={2} aria-hidden="true" />}
          {meta}
        </span>
      )}
    </div>
  );
});

interface ListViewProps {
  mode: GroupMode;
  empty?: ReactNode;
}

export function ListView({ mode, empty }: ListViewProps) {
  const { items, projectsById, selection, freshId } = useWorkspaceData();
  const handlers = useCardHandlers();
  const now = useNow();
  const groups = groupItems(items, mode, now);

  return (
    <div className={styles.scroll}>
      <div className={styles.list} role="grid" aria-label={uiCopy.regions.cards}>
        {groups.length === 0 && empty}
        {groups.map((g) => (
          <section key={g.key} className={styles.group} aria-label={g.label}>
            <h2 className={styles.groupLabel} data-key={g.key}>
              {g.label}
              <span className={styles.groupCount}>{g.items.length}</span>
            </h2>
            {g.items.map((item) => (
              <Row
                key={item.id}
                item={item}
                project={item.projectId ? projectsById[item.projectId] : undefined}
                mode={mode}
                now={now}
                selected={selection.has(item.id)}
                fresh={item.id === freshId}
                handlers={handlers}
              />
            ))}
          </section>
        ))}
      </div>
    </div>
  );
}
