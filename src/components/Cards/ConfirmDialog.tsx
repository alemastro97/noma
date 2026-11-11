import { useEffect, useRef } from "react";
import { cardsCopy } from "../../copy";
import { useConfirmStore } from "../../stores/cards";
import overlay from "../QuickAdd/Overlay.module.css";
import styles from "./Cards.module.css";
import { useFocusTrap } from "../../hooks/useFocusTrap";

/** Host for `confirmAction()`: a small modal asking before destructive actions. */
export function ConfirmDialog() {
  const request = useConfirmStore((s) => s.request);
  const answer = useConfirmStore((s) => s.answer);
  const confirmRef = useRef<HTMLButtonElement>(null);
  const dialogRef = useRef<HTMLDivElement>(null);
  useFocusTrap(dialogRef, !!request);

  useEffect(() => {
    if (!request) return;
    confirmRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        answer(false);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [request, answer]);

  if (!request) return null;
  return (
    <div className={overlay.backdrop} onMouseDown={() => answer(false)}>
      <div
        className={`${overlay.panel} ${styles.confirm}`}
        ref={dialogRef}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="confirm-title"
        aria-describedby={request.body ? "confirm-body" : undefined}
        tabIndex={-1}
        onMouseDown={(e) => e.stopPropagation()}
      >
        <h2 id="confirm-title" className={styles.confirmTitle}>
          {request.title}
        </h2>
        {request.body && (
          <p id="confirm-body" className={styles.confirmBody}>
            {request.body}
          </p>
        )}
        <div className={styles.confirmActions}>
          <button type="button" className={styles.button} onClick={() => answer(false)}>
            {cardsCopy.cancel}
          </button>
          <button
            ref={confirmRef}
            type="button"
            className={styles.button}
            data-tone={request.danger ? "danger" : "primary"}
            onClick={() => answer(true)}
          >
            {request.confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
