import { Copy, Flag, Maximize2, Pin, PinOff, Repeat, Trash2 } from "lucide-react";
import { memo, type CSSProperties, type MouseEvent } from "react";
import type { Item, Progress, Project } from "../../api/types";
import { cardCopy, cardsCopy, hintCopy, kindLabel, priorityLabel } from "../../copy";
import {
  editedAgo,
  eventBlock,
  formatDay,
  formatDue,
  formatReminder,
  relativeFromNow,
  timeRange,
  toDate,
} from "../../lib/dates";
import { describeRule } from "../../lib/recurrence";
import styles from "./Card.module.css";
import { Checkbox } from "./Checkbox";
import { HintChips } from "./HintChips";
import { NomaOrb } from "../NomaOrb";
import { CardBadges, PinMark } from "../Cards/CardBadges";
import { KIND_STYLE } from "./kind";

export type CardVariant = "canvas" | "board" | "compact";

export interface CardProps {
  item: Item;
  project?: Project;
  progress?: Progress;
  selected?: boolean;
  fresh?: boolean;
  /** canvas: fixed width per type · board: fills a Kanban column · compact: tighter. */
  variant?: CardVariant;
  now: Date;
  onToggle?: (id: string) => void;
  onOpen?: (id: string) => void;
  onSelect?: (id: string) => void;
  onDuplicate?: (id: string) => void;
  onDelete?: (id: string) => void;
  onPin?: (id: string) => void;
  className?: string;
  style?: CSSProperties;
}

function Tags({ tags }: { tags: string[] }) {
  return (
    <>
      {tags.map((t) => (
        <span key={t} className={styles.tag}>
          #{t}
        </span>
      ))}
    </>
  );
}

function PriorityFlag({ item }: { item: Item }) {
  if (item.priority !== "high" && item.priority !== "urgent") return null;
  return (
    <span className={styles.priority}>
      <Flag size={12} strokeWidth={2} aria-hidden="true" />
      {priorityLabel[item.priority]}
    </span>
  );
}

function KindLabel({ item, extra }: { item: Item; extra?: string }) {
  const { Icon, color } = KIND_STYLE[item.kind];
  return (
    <div className={styles.kind} style={{ color }}>
      <Icon size={14} strokeWidth={1.75} aria-hidden={true} />
      <span>
        {kindLabel[item.kind]}
        {extra && <span className={styles.kindExtra}> · {extra}</span>}
      </span>
      {item.recurrence && (
        <span className={styles.repeat} title={describeRule(item.recurrence)}>
          <Repeat size={12} strokeWidth={2} aria-label={cardCopy.repeats} />
        </span>
      )}
    </div>
  );
}

function Actions({
  item,
  onOpen,
  onDuplicate,
  onDelete,
  onPin,
}: Pick<CardProps, "item" | "onOpen" | "onDuplicate" | "onDelete" | "onPin">) {
  const stop = (fn?: (id: string) => void) => (e: MouseEvent) => {
    e.stopPropagation();
    fn?.(item.id);
  };
  return (
    <div className={styles.actions} onDoubleClick={(e) => e.stopPropagation()}>
      {onOpen && (
        <button
          type="button"
          className={styles.act}
          aria-label={cardCopy.open}
          onClick={stop(onOpen)}
        >
          <Maximize2 size={14} strokeWidth={1.75} aria-hidden="true" />
        </button>
      )}
      {onPin && (
        <button
          type="button"
          className={styles.act}
          aria-label={item.pinned ? cardsCopy.unpin : cardsCopy.pin}
          title={item.pinned ? cardsCopy.unpin : cardsCopy.pin}
          onClick={stop(onPin)}
        >
          {item.pinned ? (
            <PinOff size={14} strokeWidth={1.75} aria-hidden="true" />
          ) : (
            <Pin size={14} strokeWidth={1.75} aria-hidden="true" />
          )}
        </button>
      )}
      {onDuplicate && (
        <button
          type="button"
          className={styles.act}
          aria-label={cardCopy.duplicate}
          onClick={stop(onDuplicate)}
        >
          <Copy size={14} strokeWidth={1.75} aria-hidden="true" />
        </button>
      )}
      {onDelete && (
        <button
          type="button"
          className={styles.act}
          aria-label={cardCopy.delete}
          onClick={stop(onDelete)}
        >
          <Trash2 size={14} strokeWidth={1.75} aria-hidden="true" />
        </button>
      )}
    </div>
  );
}

function ProjectChip({ project }: { project?: Project }) {
  if (!project) return null;
  return <span className={styles.tag}>{project.name}</span>;
}

function Body({ item, project, progress, now, onToggle }: CardProps) {
  const done = item.status === "done";
  const due = toDate(item.dueAt);
  const remind = toDate(item.remindAt);

  switch (item.kind) {
    case "task":
      return (
        <div className={styles.taskRow}>
          <Checkbox checked={done} onToggle={() => onToggle?.(item.id)} />
          <div className={styles.stack}>
            <h3 className={styles.title}>{item.title}</h3>
            {(due || remind || project || item.tags.length > 0 || item.priority !== "normal") && (
              <div className={styles.row}>
                {due && (
                  <span
                    className={styles.meta}
                    data-overdue={!done && due.getTime() < now.getTime()}
                  >
                    {formatDue(due, now)}
                  </span>
                )}
                <PriorityFlag item={item} />
                <ProjectChip project={project} />
                {remind && (
                  <span className={styles.remindChip}>
                    <KindReminderIcon />
                    {formatDay(remind, now)}
                  </span>
                )}
                <Tags tags={item.tags} />
              </div>
            )}
          </div>
        </div>
      );

    case "reminder": {
      const at = remind ?? due;
      return (
        <>
          <div className={styles.spread}>
            <KindLabel item={item} />
            {at && !done && <span className={styles.meta}>{relativeFromNow(at, now)}</span>}
          </div>
          {at && <div className={styles.reminderTime}>{formatReminder(at, now)}</div>}
          <div className={styles.taskRow}>
            <Checkbox
              checked={done}
              color="var(--noma-reminder)"
              size={15}
              onToggle={() => onToggle?.(item.id)}
            />
            <h3 className={styles.title}>{item.title}</h3>
          </div>
        </>
      );
    }

    case "note": {
      const edited = toDate(item.updatedAt);
      return (
        <>
          <KindLabel item={item} extra={edited ? editedAgo(edited, now) : undefined} />
          <h3 className={styles.title} data-size="note">
            {item.title}
          </h3>
          {item.body && <p className={styles.preview}>{item.body}</p>}
          {(item.tags.length > 0 || project) && (
            <div className={styles.row}>
              <ProjectChip project={project} />
              <Tags tags={item.tags} />
            </div>
          )}
        </>
      );
    }

    case "idea":
      return (
        <>
          <KindLabel item={item} />
          <h3 className={styles.ideaTitle}>{item.title}</h3>
          {(item.linkCount > 0 || item.tags.length > 0) && (
            <div className={styles.row}>
              {item.linkCount > 0 && (
                <span className={styles.meta}>{cardCopy.linked(item.linkCount)}</span>
              )}
              <Tags tags={item.tags} />
            </div>
          )}
        </>
      );

    case "project_ref": {
      const color = project?.color ?? "var(--noma-project)";
      const pct = progress && progress.total > 0 ? (progress.done / progress.total) * 100 : 0;
      return (
        <>
          <div className={styles.spread}>
            <KindLabel item={item} />
            {due && <span className={styles.meta}>due {formatDay(due, now)}</span>}
          </div>
          <h3 className={styles.title} data-size="project">
            {item.title}
          </h3>
          {item.body && <p className={styles.previewShort}>{item.body}</p>}
          <div className={styles.progressRow}>
            <div className={styles.progressTrack}>
              <div
                className={styles.progressFill}
                style={{ width: `${pct}%`, background: color }}
              />
            </div>
            <span className={styles.meta}>
              {progress?.done ?? 0}/{progress?.total ?? 0}
            </span>
          </div>
        </>
      );
    }

    case "event": {
      const start = due ?? remind;
      const block = start ? eventBlock(start) : null;
      return (
        <div className={styles.eventRow}>
          {block && (
            <div className={styles.dateBlock}>
              <span className={styles.dateWeekday}>{block.weekday}</span>
              <span className={styles.dateDay}>{block.day}</span>
            </div>
          )}
          <div className={styles.stack} style={{ gap: 6 }}>
            <h3 className={styles.title} data-size="note">
              {item.title}
            </h3>
            {start && <span className={styles.meta}>{timeRange(start, toDate(item.endsAt))}</span>}
          </div>
          {project && (
            <span
              className={styles.eventDot}
              style={{ background: project.color }}
              aria-hidden="true"
            />
          )}
        </div>
      );
    }
  }
}

function KindReminderIcon() {
  const { Icon } = KIND_STYLE.reminder;
  return <Icon size={12} strokeWidth={2} aria-hidden={true} />;
}

/** A NOMA card: one shell, six types. See design/screens/Cards.html. */
export const Card = memo(function Card(props: CardProps) {
  const { item, selected, fresh, variant = "canvas", onSelect, onOpen, className, style } = props;
  const done = item.status === "done";
  return (
    <article
      className={[styles.card, className].filter(Boolean).join(" ")}
      data-kind={item.kind}
      data-variant={variant}
      data-selected={selected || undefined}
      data-done={done || undefined}
      data-fresh={fresh || undefined}
      data-capturing={item.capturing || undefined}
      aria-label={item.title}
      aria-selected={selected}
      tabIndex={-1}
      style={style}
      onClick={() => onSelect?.(item.id)}
      onDoubleClick={() => onOpen?.(item.id)}
    >
      {item.capturing ? (
        <>
          <div className={styles.reading}>
            <NomaOrb size={14} state="processing" label={hintCopy.reading} />
            <span>{hintCopy.reading}</span>
          </div>
          <h3 className={styles.title}>{item.title}</h3>
        </>
      ) : (
        <>
          <PinMark item={item} />
          <Body {...props} />
          <CardBadges item={item} />
          <HintChips item={item} />
        </>
      )}
      <Actions {...props} />
    </article>
  );
});
