import { GitMerge, ListTree } from "lucide-react";
import { breakDown, mergeItems } from "../../api/organize";
import type { CaptureHint, Item } from "../../api/types";
import { organizeCopy as t } from "../../copy";
import { useItemsStore } from "../../stores/items";
import { useToastsStore } from "../../stores/toasts";
import { useUiStore } from "../../stores/ui";
import styles from "./Organize.module.css";
import { useAiAction } from "./useAiAction";

/** The hint without its "Looks like …" part (null when nothing else is left). */
export function withoutSimilar(hint: CaptureHint): CaptureHint | null {
  const next = { ...hint };
  delete next.similarTo;
  delete next.similarTitle;
  return next.datePhrase || next.lowConfidence || next.suggestedProject ? next : null;
}

/** Detail-panel block: the duplicate prompt and "Break down" for tasks. */
export function OrganizeActions({ item }: { item: Item }) {
  const merge = useAiAction(mergeItems, t.duplicate.failed);
  const steps = useAiAction(breakDown);
  const similarTo = item.hint?.similarTo;
  const showBreakDown = item.kind === "task" && item.status !== "done";
  if (!similarTo && !showBreakDown) return null;

  const onMerge = async () => {
    if (!similarTo) return;
    const merged = await merge.go(item.id, similarTo);
    if (!merged) return;
    useItemsStore.setState((s) => {
      const items = { ...s.items, [merged.id]: merged };
      delete items[item.id];
      return { items, order: s.order.filter((id) => id !== item.id) };
    });
    useUiStore.getState().openDetail(merged.id);
    useToastsStore.getState().push({ message: t.duplicate.merged });
  };

  const onKeepBoth = () => {
    if (item.hint)
      void useItemsStore.getState().update(item.id, { hint: withoutSimilar(item.hint) });
  };

  const onBreakDown = async () => {
    const ids = await steps.go(item.id);
    if (ids) {
      void useItemsStore.getState().reload();
      useToastsStore.getState().push({ message: t.breakDown.done(ids.length) });
    }
  };

  return (
    <>
      {similarTo && (
        <div className={styles.duplicate}>
          <p className={styles.duplicateText}>
            {t.duplicate.lead} <strong>{item.hint?.similarTitle}</strong>
          </p>
          <div className={styles.actions}>
            <button
              type="button"
              className={styles.primary}
              disabled={merge.busy}
              onClick={() => void onMerge()}
            >
              <GitMerge size={13} strokeWidth={1.9} aria-hidden="true" />
              {t.duplicate.merge}
            </button>
            <button type="button" className={styles.ghost} onClick={onKeepBoth}>
              {t.duplicate.keepBoth}
            </button>
          </div>
        </div>
      )}
      {showBreakDown && (
        <div className={styles.actions}>
          <button
            type="button"
            className={styles.ghost}
            title={t.breakDown.title}
            disabled={steps.busy}
            onClick={() => void onBreakDown()}
          >
            <ListTree size={13} strokeWidth={1.9} aria-hidden="true" />
            {steps.busy ? t.breakDown.working : t.breakDown.button}
          </button>
        </div>
      )}
    </>
  );
}
