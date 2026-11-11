import { RefreshCw } from "lucide-react";
import { resortInbox } from "../../api/organize";
import { organizeCopy as t } from "../../copy";
import { useToastsStore } from "../../stores/toasts";
import styles from "./Organize.module.css";
import { useAiAction } from "./useAiAction";

/** "Re-sort Inbox": files captured Inbox cards again with today's projects and boards. */
export function InboxResort() {
  const { busy, go } = useAiAction(resortInbox);
  const onClick = async () => {
    const result = await go();
    if (result) useToastsStore.getState().push({ message: t.resort.moved(result.moved) });
  };
  return (
    <div className={styles.strip}>
      <div className={styles.stripRow}>
        <span className={styles.spacer} />
        <button
          type="button"
          className={styles.ghost}
          title={t.resort.title}
          disabled={busy}
          onClick={() => void onClick()}
        >
          <RefreshCw
            size={13}
            strokeWidth={1.9}
            aria-hidden="true"
            className={busy ? styles.spin : undefined}
          />
          {busy ? t.resort.working : t.resort.button}
        </button>
      </div>
    </div>
  );
}
