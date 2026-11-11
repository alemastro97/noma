import { Archive, Pin, PinOff, RotateCcw } from "lucide-react";
import { deleteForever } from "../../api/cards";
import type { Item } from "../../api/types";
import { cardsCopy } from "../../copy";
import {
  archiveCard,
  confirmAction,
  restoreCard,
  setPinned,
  unarchiveCard,
} from "../../stores/cards";
import { toastError } from "../../stores/toasts";
import { useUiStore } from "../../stores/ui";
import panel from "../DetailPanel/DetailPanel.module.css";
import styles from "./Cards.module.css";

/** Pin and Archive buttons for the detail panel header. */
export function CardHeaderActions({ item, onClose }: { item: Item; onClose: () => void }) {
  if (item.deletedAt) return null;
  return (
    <>
      <button
        type="button"
        className={panel.iconBtn}
        aria-label={item.pinned ? cardsCopy.unpin : cardsCopy.pin}
        title={item.pinned ? cardsCopy.unpin : cardsCopy.pin}
        aria-pressed={Boolean(item.pinned)}
        onClick={() => void setPinned(item.id, !item.pinned)}
      >
        {item.pinned ? (
          <PinOff size={15} strokeWidth={1.75} aria-hidden="true" />
        ) : (
          <Pin size={15} strokeWidth={1.75} aria-hidden="true" />
        )}
      </button>
      <button
        type="button"
        className={panel.iconBtn}
        aria-label={item.archivedAt ? cardsCopy.unarchive : cardsCopy.archive}
        title={item.archivedAt ? cardsCopy.unarchive : cardsCopy.archive}
        onClick={() => {
          if (item.archivedAt) void unarchiveCard(item);
          else {
            void archiveCard(item);
            onClose();
          }
        }}
      >
        {item.archivedAt ? (
          <RotateCcw size={15} strokeWidth={1.75} aria-hidden="true" />
        ) : (
          <Archive size={15} strokeWidth={1.75} aria-hidden="true" />
        )}
      </button>
    </>
  );
}

/** Tells the user the card is archived or in the Trash, with a way back. */
export function CardStateBanner({ item }: { item: Item }) {
  if (item.deletedAt) {
    return (
      <div className={styles.banner} role="status">
        <span>{cardsCopy.inTrash}</span>
        <button type="button" className={styles.button} onClick={() => void restoreCard(item)}>
          {cardsCopy.restore}
        </button>
        <button
          type="button"
          className={styles.button}
          data-tone="danger"
          onClick={() => {
            void confirmAction({
              title: cardsCopy.deleteForeverTitle,
              body: cardsCopy.deleteForeverBody,
              confirmLabel: cardsCopy.deleteForever,
              danger: true,
            }).then((ok) => {
              if (!ok) return;
              useUiStore.getState().closeDetail();
              deleteForever(item.id).catch((err: unknown) => toastError(String(err)));
            });
          }}
        >
          {cardsCopy.deleteForever}
        </button>
      </div>
    );
  }
  if (item.archivedAt) {
    return (
      <div className={styles.banner} role="status">
        <span>{cardsCopy.inArchive}</span>
        <button type="button" className={styles.button} onClick={() => void unarchiveCard(item)}>
          {cardsCopy.restore}
        </button>
      </div>
    );
  }
  return null;
}
