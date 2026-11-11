import { Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { onItemsChanged } from "../../api/events";
import { getReview, reviewSummary, type Review } from "../../api/organize";
import type { Item } from "../../api/types";
import { KIND_STYLE } from "../../components/Card/kind";
import { organizeCopy } from "../../copy";
import { anchorDate, formatDue, toDate } from "../../lib/dates";
import { useUiStore } from "../../stores/ui";
import styles from "./Organize.module.css";
import { useAiAction } from "./useAiAction";

type SectionKey = "completed" | "overdue" | "forgotten";

function Row({ item, section, now }: { item: Item; section: SectionKey; now: Date }) {
  const date =
    section === "completed"
      ? toDate(item.completedAt)
      : section === "overdue"
        ? anchorDate(item)
        : toDate(item.updatedAt);
  return (
    <button
      type="button"
      className={styles.row}
      data-done={section === "completed" || undefined}
      onClick={() => useUiStore.getState().openDetail(item.id)}
    >
      <span
        className={styles.rowDot}
        style={{ background: KIND_STYLE[item.kind].color }}
        aria-hidden="true"
      />
      <span className={styles.rowTitle}>{item.title}</span>
      {date && <span className={styles.rowMeta}>{formatDue(date, now)}</span>}
    </button>
  );
}

function Section({
  id,
  label,
  hint,
  empty,
  items,
  now,
}: {
  id: SectionKey;
  label: string;
  hint?: string;
  empty: string;
  items: Item[];
  now: Date;
}) {
  return (
    <section className={styles.section} aria-label={label}>
      <h2 className={styles.sectionHead} data-key={id}>
        {label}
        <span className={styles.count}>{items.length}</span>
        {hint && <span className={styles.count}>· {hint}</span>}
      </h2>
      {items.length === 0 ? (
        <p className={styles.empty}>{empty}</p>
      ) : (
        items.map((item) => <Row key={item.id} item={item} section={id} now={now} />)
      )}
    </section>
  );
}

/** Weekly review: done in the last 7 days, overdue, forgotten, and a short summary. */
export function ReviewView() {
  const [review, setReview] = useState<Review | null>(null);
  const [summary, setSummary] = useState<string | null>(null);
  const { busy, go } = useAiAction(reviewSummary);

  useEffect(() => {
    let alive = true;
    const load = () =>
      void getReview()
        .then((r) => alive && setReview(r))
        .catch(() => undefined);
    load();
    const un = onItemsChanged(load);
    return () => {
      alive = false;
      void un.then((f) => f()).catch(() => undefined);
    };
  }, []);

  const now = new Date();
  const write = async () => {
    const s = await go();
    if (s) setSummary(s);
  };

  return (
    <div className={styles.review}>
      <div className={styles.reviewInner}>
        <p className={styles.lead}>{organizeCopy.review.lead}</p>

        <div className={styles.brief}>
          <div className={styles.next}>
            <span className={styles.nextLabel}>{organizeCopy.review.summary}</span>
            <span className={styles.spacer} />
            <button
              type="button"
              className={summary ? styles.ghost : styles.primary}
              disabled={busy || !review}
              onClick={() => void write()}
            >
              <Sparkles size={13} strokeWidth={1.9} aria-hidden="true" />
              {busy
                ? organizeCopy.review.working
                : summary
                  ? organizeCopy.review.resummarize
                  : organizeCopy.review.summarize}
            </button>
          </div>
          {summary && <p className={styles.summary}>{summary}</p>}
        </div>

        {!review ? (
          <p className={styles.empty}>{organizeCopy.review.loading}</p>
        ) : (
          <>
            <Section
              id="overdue"
              label={organizeCopy.review.overdue}
              empty={organizeCopy.review.emptyOverdue}
              items={review.overdue}
              now={now}
            />
            <Section
              id="completed"
              label={organizeCopy.review.completed}
              empty={organizeCopy.review.emptyCompleted}
              items={review.completed}
              now={now}
            />
            <Section
              id="forgotten"
              label={organizeCopy.review.forgotten}
              hint={organizeCopy.review.forgottenHint}
              empty={organizeCopy.review.emptyForgotten}
              items={review.forgotten}
              now={now}
            />
          </>
        )}
      </div>
    </div>
  );
}
