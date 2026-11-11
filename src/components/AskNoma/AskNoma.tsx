import { ArrowUp, RotateCcw, Sparkles, X } from "lucide-react";
import { Fragment, useEffect, useRef, useState, type ReactNode } from "react";
import type { Item } from "../../api/types";
import { kindLabel, semanticCopy as t } from "../../copy";
import { useAskStore, type Turn } from "../../stores/ask";
import { useUiStore } from "../../stores/ui";
import { KIND_STYLE } from "../Card/kind";
import overlay from "../QuickAdd/Overlay.module.css";
import styles from "./AskNoma.module.css";

/** Shows a cited card in its place, like "Open" on a filed toast. */
function openCard(item: Item) {
  const ui = useUiStore.getState();
  ui.setTarget(item.projectId ? { kind: "project", id: item.projectId } : { kind: "inbox" });
  ui.openDetail(item.id);
  useAskStore.getState().close();
}

/** The answer text with `[n]` markers turned into clickable citations. */
export function renderAnswer(answer: string, cards: Item[]): ReactNode[] {
  return answer.split(/(\[\d+\])/g).map((part, i) => {
    const m = /^\[(\d+)\]$/.exec(part);
    const card = m ? cards[Number(m[1]) - 1] : undefined;
    if (!m || !card) return <Fragment key={i}>{part}</Fragment>;
    return (
      <button
        key={i}
        type="button"
        className={styles.cite}
        title={card.title}
        aria-label={t.openCard(card.title)}
        onClick={() => openCard(card)}
      >
        {m[1]}
      </button>
    );
  });
}

function SourceCard({ n, item }: { n: number; item: Item }) {
  const { Icon, color } = KIND_STYLE[item.kind];
  return (
    <button
      type="button"
      className={styles.source}
      onClick={() => openCard(item)}
      aria-label={t.openCard(item.title)}
    >
      <span className={styles.sourceNum}>{n}</span>
      <span style={{ color, display: "inline-flex" }} aria-hidden="true">
        <Icon size={14} strokeWidth={1.75} />
      </span>
      <span className={styles.sourceTitle}>{item.title}</span>
      <span className={styles.sourceKind}>{kindLabel[item.kind]}</span>
    </button>
  );
}

function TurnView({ turn }: { turn: Turn }) {
  const result = turn.result;
  const cited = result
    ? result.cited.flatMap((n) => {
        const item = result.cards[n - 1];
        return item ? [{ n, item }] : [];
      })
    : [];
  return (
    <div className={styles.turn}>
      <div className={styles.question}>{turn.question}</div>
      <div className={styles.answer} aria-live="polite">
        {turn.pending ? (
          <span className={styles.thinking}>{t.thinking}</span>
        ) : turn.error ? (
          <span className={styles.error}>{t.failed(turn.error)}</span>
        ) : result && result.answer ? (
          <p className={styles.text}>{renderAnswer(result.answer, result.cards)}</p>
        ) : (
          <span className={styles.muted}>{t.noCards}</span>
        )}
      </div>
      {cited.length > 0 && (
        <div className={styles.sources}>
          <span className={styles.sourcesLabel}>{t.sources}</span>
          {cited.map(({ n, item }) => (
            <SourceCard key={n} n={n} item={item} />
          ))}
        </div>
      )}
    </div>
  );
}

export function AskNoma() {
  const open = useAskStore((s) => s.open);
  return open ? <AskDialog /> : null;
}

function AskDialog() {
  const turns = useAskStore((s) => s.turns);
  const draft = useAskStore((s) => s.draft);
  const { ask, close, clear } = useAskStore.getState();
  const [text, setText] = useState(draft);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const busy = turns.some((x) => x.pending);

  useEffect(() => {
    requestAnimationFrame(() => inputRef.current?.focus());
  }, []);

  useEffect(() => {
    const el = listRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [turns]);

  const submit = (q: string) => {
    if (!q.trim() || busy) return;
    setText("");
    void ask(q);
  };

  return (
    <div className={overlay.backdrop} onMouseDown={close}>
      <div
        className={`${overlay.panel} ${styles.panel}`}
        role="dialog"
        aria-label={t.title}
        onMouseDown={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            close();
          }
        }}
      >
        <div className={styles.head}>
          <Sparkles size={16} strokeWidth={1.75} aria-hidden="true" className={styles.headIcon} />
          <h2 className={styles.title}>{t.title}</h2>
          {turns.length > 0 && (
            <button type="button" className={styles.iconButton} onClick={clear} disabled={busy}>
              <RotateCcw size={14} strokeWidth={1.75} aria-hidden="true" />
              <span>{t.newConversation}</span>
            </button>
          )}
          <button type="button" className={styles.iconButton} onClick={close} aria-label={t.close}>
            <X size={15} strokeWidth={1.75} aria-hidden="true" />
          </button>
        </div>

        <div className={styles.list} ref={listRef}>
          {turns.length === 0 ? (
            <div className={styles.intro}>
              <p>{t.intro}</p>
              <div className={styles.examples}>
                {t.examples.map((ex) => (
                  <button
                    key={ex}
                    type="button"
                    className={styles.example}
                    onClick={() => submit(ex)}
                  >
                    {ex}
                  </button>
                ))}
              </div>
            </div>
          ) : (
            turns.map((turn) => <TurnView key={turn.id} turn={turn} />)
          )}
        </div>

        <form
          className={styles.inputRow}
          onSubmit={(e) => {
            e.preventDefault();
            submit(text);
          }}
        >
          <input
            ref={inputRef}
            className={overlay.input}
            placeholder={t.placeholder}
            aria-label={t.placeholder}
            value={text}
            onChange={(e) => setText(e.target.value)}
          />
          <button
            type="submit"
            className={styles.send}
            disabled={busy || !text.trim()}
            aria-label={t.send}
          >
            <ArrowUp size={16} strokeWidth={2} aria-hidden="true" />
          </button>
        </form>
      </div>
    </div>
  );
}
