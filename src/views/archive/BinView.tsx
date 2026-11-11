import { LayoutGrid } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import * as cardsApi from "../../api/cards";
import { onItemsChanged } from "../../api/events";
import type { Board, Item, Project } from "../../api/types";
import { KIND_STYLE } from "../../components/Card/kind";
import styles from "../../components/Cards/Cards.module.css";
import { EmptyState } from "../../components/EmptyState/EmptyState";
import { cardsCopy } from "../../copy";
import { confirmAction, restoreCard, unarchiveCard } from "../../stores/cards";
import { toastError } from "../../stores/toasts";
import { useUiStore } from "../../stores/ui";

export type BinMode = "archive" | "trash";

interface BinData {
  items: Item[];
  projects: Project[];
  boards: Board[];
}

const EMPTY: BinData = { items: [], projects: [], boards: [] };

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function daysSince(iso: string | null | undefined, now: Date): number {
  if (!iso) return 0;
  return Math.floor((now.getTime() - new Date(iso).getTime()) / 86_400_000);
}

function load(mode: BinMode): Promise<BinData> {
  if (mode === "trash") return cardsApi.listTrashed().then((items) => ({ ...EMPTY, items }));
  return Promise.all([
    cardsApi.listArchived(),
    cardsApi.listArchivedProjects(),
    cardsApi.listArchivedBoards(),
  ]).then(([items, projects, boards]) => ({ items, projects, boards }));
}

function CardRow({ item, mode, now }: { item: Item; mode: BinMode; now: Date }) {
  const { Icon, color } = KIND_STYLE[item.kind];
  return (
    <li className={styles.binRow}>
      <button
        type="button"
        className={styles.binMain}
        onClick={() => useUiStore.getState().openDetail(item.id)}
      >
        <span style={{ color, display: "inline-flex" }}>
          <Icon size={15} strokeWidth={1.75} aria-hidden={true} />
        </span>
        <span className={styles.binTitle}>{item.title}</span>
        {mode === "trash" && (
          <span className={styles.sectionMeta}>
            {cardsCopy.deletedAgo(daysSince(item.deletedAt, now))}
          </span>
        )}
      </button>
      <span className={styles.binActions}>
        <button
          type="button"
          className={styles.button}
          onClick={() => void (mode === "trash" ? restoreCard(item) : unarchiveCard(item))}
        >
          {cardsCopy.restore}
        </button>
        {mode === "trash" && (
          <button
            type="button"
            className={styles.button}
            data-tone="danger"
            onClick={() =>
              void confirmAction({
                title: cardsCopy.deleteForeverTitle,
                body: cardsCopy.deleteForeverBody,
                confirmLabel: cardsCopy.deleteForever,
                danger: true,
              }).then((ok) => {
                if (ok)
                  cardsApi.deleteForever(item.id).catch((e: unknown) => toastError(message(e)));
              })
            }
          >
            {cardsCopy.deleteForever}
          </button>
        )}
      </span>
    </li>
  );
}

function PlaceRow({ name, dot, onRestore }: { name: string; dot?: string; onRestore: () => void }) {
  return (
    <li className={styles.binRow}>
      <span className={styles.binMain}>
        {dot ? (
          <span className={styles.binDot} style={{ background: dot }} aria-hidden="true" />
        ) : (
          <LayoutGrid size={15} strokeWidth={1.75} aria-hidden="true" />
        )}
        <span className={styles.binTitle}>{name}</span>
      </span>
      <span className={styles.binActions}>
        <button type="button" className={styles.button} onClick={onRestore}>
          {cardsCopy.restore}
        </button>
      </span>
    </li>
  );
}

/** The Archive and Trash views: what was put away, with a way back. */
export function BinView({ mode }: { mode: BinMode }) {
  const [data, setData] = useState<BinData>(EMPTY);
  const [now] = useState(() => new Date());

  const refresh = useCallback(() => {
    load(mode)
      .then((d) =>
        setData({ items: d.items ?? [], projects: d.projects ?? [], boards: d.boards ?? [] }),
      )
      .catch((e: unknown) => toastError(message(e)));
  }, [mode]);

  useEffect(() => {
    refresh();
    const un = onItemsChanged(() => refresh());
    return () => {
      void un.then((f) => f()).catch(() => undefined);
    };
  }, [refresh]);

  const restorePlace = (p: Promise<unknown>) =>
    void p.catch((e: unknown) => toastError(message(e)));
  const empty = mode === "trash" ? cardsCopy.trashEmpty : cardsCopy.archiveEmpty;
  const nothing = data.items.length + data.projects.length + data.boards.length === 0;

  return (
    <div className={styles.binView}>
      {mode === "trash" && (
        <div className={styles.binHead}>
          <span>{cardsCopy.trashNote}</span>
          <button
            type="button"
            className={styles.button}
            data-tone="danger"
            disabled={data.items.length === 0}
            onClick={() =>
              void confirmAction({
                title: cardsCopy.emptyTrashTitle,
                body: cardsCopy.emptyTrashBody(data.items.length),
                confirmLabel: cardsCopy.emptyTrash,
                danger: true,
              }).then((ok) => {
                if (ok) cardsApi.emptyTrash().catch((e: unknown) => toastError(message(e)));
              })
            }
          >
            {cardsCopy.emptyTrash}
          </button>
        </div>
      )}
      {nothing && <EmptyState title={empty.title} body={empty.body} />}
      {data.projects.length > 0 && (
        <section className={styles.binGroup}>
          <h2 className={styles.binLabel}>{cardsCopy.archivedProjects}</h2>
          <ul className={styles.binList}>
            {data.projects.map((p) => (
              <PlaceRow
                key={p.id}
                name={p.name}
                dot={p.color}
                onRestore={() => restorePlace(cardsApi.archiveProject(p.id, false))}
              />
            ))}
          </ul>
        </section>
      )}
      {data.boards.length > 0 && (
        <section className={styles.binGroup}>
          <h2 className={styles.binLabel}>{cardsCopy.archivedBoards}</h2>
          <ul className={styles.binList}>
            {data.boards.map((b) => (
              <PlaceRow
                key={b.id}
                name={b.name}
                onRestore={() => restorePlace(cardsApi.archiveBoard(b.id, false))}
              />
            ))}
          </ul>
        </section>
      )}
      {data.items.length > 0 && (
        <section className={styles.binGroup}>
          {mode === "archive" && <h2 className={styles.binLabel}>{cardsCopy.archivedCards}</h2>}
          <ul className={styles.binList}>
            {data.items.map((item) => (
              <CardRow key={item.id} item={item} mode={mode} now={now} />
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
