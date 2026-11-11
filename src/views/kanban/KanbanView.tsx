import { Plus } from "lucide-react";
import { useState, type DragEvent, type ReactNode } from "react";
import { STATUSES, type Status } from "../../api/types";
import { statusLabel, uiCopy } from "../../copy";
import { Card } from "../../components/Card/Card";
import { useItemsStore } from "../../stores/items";
import { useUiStore } from "../../stores/ui";
import { progressFor, useCardHandlers, useNow, useWorkspaceData } from "../useWorkspace";
import { groupByStatus } from "./group";
import styles from "./Kanban.module.css";
import { draggedIds, startCardDrag } from "../../components/Sidebar/dropTarget";
import { batchHistory } from "../../stores/history";

const DRAG_TYPE = "application/x-noma-item";

export function KanbanView({ empty }: { empty?: ReactNode }) {
  const { items, projectsById, progress, selection, freshId } = useWorkspaceData();
  const handlers = useCardHandlers();
  const now = useNow();
  const [over, setOver] = useState<Status | null>(null);
  const [dragging, setDragging] = useState<string | null>(null);
  const columns = groupByStatus(items, now);

  const onDrop = (status: Status) => (e: DragEvent) => {
    e.preventDefault();
    setOver(null);
    const store = useItemsStore.getState();
    batchHistory("bulk", () => {
      for (const id of draggedIds(e)) {
        const item = store.items[id];
        if (item && item.status !== status) void store.setStatus(id, status);
      }
    });
  };

  return (
    <div className={styles.board}>
      {items.length === 0 && empty && <div className={styles.empty}>{empty}</div>}
      {STATUSES.map((status) => (
        <section
          key={status}
          className={styles.column}
          data-over={over === status || undefined}
          aria-label={statusLabel[status]}
          onDragOver={(e) => {
            if (!e.dataTransfer.types.includes(DRAG_TYPE)) return;
            e.preventDefault();
            e.dataTransfer.dropEffect = "move";
            if (over !== status) setOver(status);
          }}
          onDragLeave={(e) => {
            if (!e.currentTarget.contains(e.relatedTarget as Node)) setOver(null);
          }}
          onDrop={onDrop(status)}
        >
          <header className={styles.columnHeader}>
            <span className={styles.dot} data-status={status} aria-hidden="true" />
            <h2 className={styles.columnTitle}>{statusLabel[status]}</h2>
            <span className={styles.count}>{columns[status].length}</span>
          </header>
          <div className={styles.cards}>
            {columns[status].map((item) => (
              <div
                key={item.id}
                className={styles.draggable}
                data-dragging={dragging === item.id || undefined}
                data-item-id={item.id}
                draggable
                onDragStart={(e) => {
                  startCardDrag(e, item.id);
                  setDragging(item.id);
                }}
                onDragEnd={() => {
                  setDragging(null);
                  setOver(null);
                }}
              >
                <Card
                  item={item}
                  project={item.projectId ? projectsById[item.projectId] : undefined}
                  progress={progressFor(item, progress)}
                  now={now}
                  variant="board"
                  selected={selection.has(item.id)}
                  fresh={item.id === freshId}
                  {...handlers}
                />
              </div>
            ))}
            {status !== "done" && (
              <button
                type="button"
                className={styles.add}
                onClick={() => useUiStore.getState().openQuickAdd({ status })}
              >
                <Plus size={14} strokeWidth={1.75} aria-hidden="true" />
                {uiCopy.add}
              </button>
            )}
          </div>
        </section>
      ))}
    </div>
  );
}
