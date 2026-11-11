import { ListChecks, Paperclip, Pin } from "lucide-react";
import type { Item } from "../../api/types";
import { cardsCopy } from "../../copy";
import styles from "./Cards.module.css";

/** The small pin in a pinned card's corner. */
export function PinMark({ item }: { item: Item }) {
  if (!item.pinned) return null;
  return (
    <span className={styles.pinMark} title={cardsCopy.pinned}>
      <Pin size={12} strokeWidth={2} aria-label={cardsCopy.pinned} />
    </span>
  );
}

/** Checklist progress and attachment count, when a card has them. */
export function CardBadges({ item }: { item: Item }) {
  const total = item.subtasksTotal ?? 0;
  const done = item.subtasksDone ?? 0;
  const files = item.attachmentCount ?? 0;
  if (total === 0 && files === 0) return null;
  return (
    <div className={styles.badges}>
      {total > 0 && (
        <span
          className={styles.badge}
          data-complete={done === total || undefined}
          title={cardsCopy.checklist}
        >
          <ListChecks size={12} strokeWidth={2} aria-hidden="true" />
          {cardsCopy.subtasksProgress(done, total)}
        </span>
      )}
      {files > 0 && (
        <span className={styles.badge} title={cardsCopy.attachmentCount(files)}>
          <Paperclip size={12} strokeWidth={2} aria-hidden="true" />
          {files}
        </span>
      )}
    </div>
  );
}
