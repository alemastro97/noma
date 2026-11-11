import { useEffect, useRef } from "react";
import { cardsCopy } from "../../copy";
import { useConfirmStore } from "../../stores/cards";
import overlay from "../QuickAdd/Overlay.module.css";
import styles from "./Cards.module.css";

/** Host for `confirmAction()`: a small modal asking before destructive actions. */
export function ConfirmDialog() {
  const request = useConfirmStore((s) => s.request);
  const answer = useConfirmStore((s) => s.answer);
  const confirmRef = useRef<HTMLButtonElement>(null);

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
        role="alertdialog"
        aria-modal="true"
        aria-label={request.title}
        onMouseDown={(e) => e.stopPropagation()}
      >
        <h2 className={styles.confirmTitle}>{request.title}</h2>
        {request.body && <p className={styles.confirmBody}>{request.body}</p>}
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
