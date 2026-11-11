import { ArrowDown, ArrowUp, Plus, X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import * as cardsApi from "../../api/cards";
import type { Subtask } from "../../api/cards";
import { cardsCopy } from "../../copy";
import { toastError } from "../../stores/toasts";
import { Checkbox } from "../Card/Checkbox";
import styles from "./Cards.module.css";

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function SubtaskRow({
  subtask,
  first,
  last,
  onToggle,
  onRename,
  onRemove,
  onMove,
}: {
  subtask: Subtask;
  first: boolean;
  last: boolean;
  onToggle: () => void;
  onRename: (title: string) => void;
  onRemove: () => void;
  onMove: (delta: -1 | 1) => void;
}) {
  const [draft, setDraft] = useState(subtask.title);
  const [synced, setSynced] = useState(subtask.title);
  if (subtask.title !== synced) {
    setSynced(subtask.title);
    setDraft(subtask.title);
  }
  const commit = () => {
    const t = draft.trim();
    if (!t) setDraft(subtask.title);
    else if (t !== subtask.title) onRename(t);
  };
  return (
    <li className={styles.subtask} data-done={subtask.done || undefined}>
      <Checkbox checked={subtask.done} size={16} onToggle={onToggle} />
      <input
        className={styles.subtaskInput}
        value={draft}
        aria-label={subtask.title}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") (e.target as HTMLInputElement).blur();
          else if (e.key === "Escape") {
            setDraft(subtask.title);
            (e.target as HTMLInputElement).blur();
          }
        }}
      />
      <span className={styles.rowActions}>
        {!first && (
          <button
            type="button"
            className={styles.miniBtn}
            aria-label={cardsCopy.moveUp}
            title={cardsCopy.moveUp}
            onClick={() => onMove(-1)}
          >
            <ArrowUp size={12} strokeWidth={2} aria-hidden="true" />
          </button>
        )}
        {!last && (
          <button
            type="button"
            className={styles.miniBtn}
            aria-label={cardsCopy.moveDown}
            title={cardsCopy.moveDown}
            onClick={() => onMove(1)}
          >
            <ArrowDown size={12} strokeWidth={2} aria-hidden="true" />
          </button>
        )}
        <button
          type="button"
          className={styles.miniBtn}
          aria-label={cardsCopy.removeSubtask}
          title={cardsCopy.removeSubtask}
          onClick={onRemove}
        >
          <X size={12} strokeWidth={2} aria-hidden="true" />
        </button>
      </span>
    </li>
  );
}

/** The checklist section of the detail panel. */
export function Subtasks({ itemId, version }: { itemId: string; version: string }) {
  const [list, setList] = useState<Subtask[]>([]);
  const [text, setText] = useState("");

  const load = useCallback(() => {
    cardsApi
      .listSubtasks(itemId)
      .then((l) => setList(l ?? []))
      .catch(() => undefined);
  }, [itemId]);

  // Reload when the card changes (another window, a toggle elsewhere).
  useEffect(() => load(), [load, version]);

  const run = (p: Promise<unknown>) =>
    void p.catch((err: unknown) => {
      toastError(message(err));
      load();
    });

  const add = () => {
    const title = text.trim();
    setText("");
    if (!title) return;
    run(cardsApi.addSubtask(itemId, title).then((s) => setList((l) => [...l, s])));
  };

  const toggle = (s: Subtask) => {
    setList((l) => l.map((x) => (x.id === s.id ? { ...x, done: !s.done } : x)));
    run(cardsApi.updateSubtask(s.id, { done: !s.done }));
  };

  const move = (index: number, delta: -1 | 1) => {
    const next = [...list];
    const [moved] = next.splice(index, 1);
    next.splice(index + delta, 0, moved!);
    setList(next);
    run(
      cardsApi.reorderSubtasks(
        itemId,
        next.map((s) => s.id),
      ),
    );
  };

  const done = list.filter((s) => s.done).length;

  return (
    <section className={styles.section} aria-label={cardsCopy.checklist}>
      <div className={styles.sectionHead}>
        <span>{cardsCopy.checklist}</span>
        {list.length > 0 && (
          <span className={styles.sectionMeta}>
            {cardsCopy.subtasksProgress(done, list.length)}
          </span>
        )}
      </div>
      {list.length > 0 && (
        <div className={styles.progressTrack} aria-hidden="true">
          <div
            className={styles.progressFill}
            style={{ width: `${(done / list.length) * 100}%` }}
          />
        </div>
      )}
      <ul className={styles.subtasks}>
        {list.map((s, i) => (
          <SubtaskRow
            key={s.id}
            subtask={s}
            first={i === 0}
            last={i === list.length - 1}
            onToggle={() => toggle(s)}
            onRename={(title) => {
              setList((l) => l.map((x) => (x.id === s.id ? { ...x, title } : x)));
              run(cardsApi.updateSubtask(s.id, { title }));
            }}
            onRemove={() => {
              setList((l) => l.filter((x) => x.id !== s.id));
              run(cardsApi.deleteSubtask(s.id));
            }}
            onMove={(delta) => move(i, delta)}
          />
        ))}
      </ul>
      <div className={styles.addRow}>
        <Plus size={14} strokeWidth={1.75} aria-hidden="true" />
        <input
          className={styles.subtaskInput}
          value={text}
          placeholder={cardsCopy.addSubtask}
          aria-label={cardsCopy.addSubtask}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              add();
            }
          }}
          onBlur={add}
        />
      </div>
    </section>
  );
}
