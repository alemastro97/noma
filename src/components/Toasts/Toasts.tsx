import { X } from "lucide-react";
import { useToastsStore } from "../../stores/toasts";
import styles from "./Toasts.module.css";

export function Toasts() {
  const toasts = useToastsStore((s) => s.toasts);
  const dismiss = useToastsStore((s) => s.dismiss);
  if (toasts.length === 0) return null;
  return (
    <div className={styles.stack} role="status" aria-live="polite">
      {toasts.map((t) => (
        <div key={t.id} className={styles.toast} data-tone={t.tone ?? "default"}>
          <span className={styles.message}>{t.message}</span>
          {t.action && (
            <button
              type="button"
              className={styles.action}
              onClick={() => {
                t.action?.run();
                dismiss(t.id);
              }}
            >
              {t.action.label}
            </button>
          )}
          <button
            type="button"
            className={styles.close}
            aria-label="Dismiss"
            onClick={() => dismiss(t.id)}
          >
            <X size={13} strokeWidth={1.75} aria-hidden="true" />
          </button>
        </div>
      ))}
    </div>
  );
}
