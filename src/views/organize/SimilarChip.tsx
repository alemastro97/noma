import { Copy, X } from "lucide-react";
import type { CaptureHint, Item } from "../../api/types";
import styles from "../../components/Card/Card.module.css";
import { hintCopy, organizeCopy } from "../../copy";
import { useItemsStore } from "../../stores/items";
import { useUiStore } from "../../stores/ui";
import { withoutSimilar } from "./OrganizeActions";

const short = (s: string) => (s.length > 36 ? `${s.slice(0, 35)}…` : s);

/** "Looks like …" on a card: opens the detail panel, where Merge / Keep both live. */
export function SimilarChip({ item, hint }: { item: Item; hint: CaptureHint }) {
  return (
    <span className={styles.hint} data-tone="project">
      <button
        type="button"
        className={styles.hintAction}
        onClick={(e) => {
          e.stopPropagation();
          useUiStore.getState().openDetail(item.id);
        }}
      >
        <Copy size={12} strokeWidth={2} aria-hidden="true" />
        {organizeCopy.duplicate.chip(short(hint.similarTitle ?? ""))}
      </button>
      <button
        type="button"
        aria-label={hintCopy.dismiss}
        onClick={(e) => {
          e.stopPropagation();
          void useItemsStore.getState().update(item.id, { hint: withoutSimilar(hint) });
        }}
      >
        <X size={11} strokeWidth={2} aria-hidden="true" />
      </button>
    </span>
  );
}
