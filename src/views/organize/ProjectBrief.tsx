import { FileText, RefreshCw, Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { onItemsChanged } from "../../api/events";
import {
  getBrief,
  suggestNextAction,
  summarizeProject,
  type ProjectBrief as Brief,
} from "../../api/organize";
import { organizeCopy as t } from "../../copy";
import { useUiStore } from "../../stores/ui";
import styles from "./Organize.module.css";
import { useAiAction } from "./useAiAction";

/**
 * The project header's brief: the suggested next action and the AI-written
 * status paragraph, each regenerable.
 */
export function ProjectBrief({ projectId }: { projectId: string }) {
  const [brief, setBrief] = useState<Brief | null>(null);
  const next = useAiAction(suggestNextAction);
  const summary = useAiAction(summarizeProject);

  useEffect(() => {
    let alive = true;
    const load = () =>
      void getBrief(projectId)
        .then((b) => alive && setBrief(b))
        .catch(() => undefined);
    load();
    // Finishing the next action's card makes it stale: reload on changes.
    const un = onItemsChanged(load);
    return () => {
      alive = false;
      void un.then((f) => f()).catch(() => undefined);
    };
  }, [projectId]);

  const run = async (action: typeof next.go) => {
    const b = await action(projectId);
    if (b) setBrief(b);
  };
  const busy = next.busy || summary.busy;

  return (
    <div className={styles.strip}>
      <div className={styles.brief}>
        <div className={styles.next}>
          <span className={styles.nextLabel}>{t.brief.next}</span>
          {brief?.nextAction ? (
            <button
              type="button"
              className={styles.nextText}
              title={t.brief.open}
              onClick={() => brief.nextItemId && useUiStore.getState().openDetail(brief.nextItemId)}
            >
              {brief.nextAction}
            </button>
          ) : (
            <span className={styles.muted}>
              {brief?.nextAt ? t.brief.nothingOpen : next.busy ? t.brief.working : ""}
            </span>
          )}
          <span className={styles.spacer} />
          {brief?.nextAction || brief?.nextAt ? (
            <button
              type="button"
              className={styles.iconBtn}
              aria-label={t.brief.refreshNext}
              title={t.brief.refreshNext}
              disabled={busy}
              onClick={() => void run(next.go)}
            >
              <RefreshCw
                size={14}
                strokeWidth={1.8}
                aria-hidden="true"
                className={next.busy ? styles.spin : undefined}
              />
            </button>
          ) : (
            <button
              type="button"
              className={styles.ghost}
              disabled={busy}
              onClick={() => void run(next.go)}
            >
              <Sparkles size={13} strokeWidth={1.9} aria-hidden="true" />
              {next.busy ? t.brief.working : t.brief.suggest}
            </button>
          )}
          <button
            type="button"
            className={styles.ghost}
            title={brief?.summary ? t.brief.resummarize : t.brief.summarize}
            disabled={busy}
            onClick={() => void run(summary.go)}
          >
            <FileText size={13} strokeWidth={1.9} aria-hidden="true" />
            {summary.busy
              ? t.brief.working
              : brief?.summary
                ? t.brief.resummarize
                : t.brief.summarize}
          </button>
        </div>
        {brief?.summary && <p className={styles.summary}>{brief.summary}</p>}
      </div>
    </div>
  );
}
