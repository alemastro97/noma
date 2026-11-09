import { Plus } from "lucide-react";
import { useState, type DragEvent, type ReactNode } from "react";
import { STATUSES, type Status } from "../../api/types";
import { statusLabel } from "../../copy";
import { Card } from "../../components/Card/Card";
import { useItemsStore } from "../../stores/items";
import { useUiStore } from "../../stores/ui";
import { progressFor, useCardHandlers, useNow, useWorkspaceData } from "../useWorkspace";
import { groupByStatus } from "./group";
import styles from "./Kanban.module.css";

const DRAG_TYPE = "application/x-noma-item";

export function KanbanView({ empty }: { empty?: ReactNode }) {
  const { items, projectsById, progress, selectedId, freshId } = useWorkspaceData();
  const handlers = useCardHandlers();
  const now = useNow();
  const [over, setOver] = useState<Status | null>(null);
  const [dragging, setDragging] = useState<string | null>(null);
  const columns = groupByStatus(items, now);

  const onDrop = (status: Status) => (e: DragEvent) => {
    e.preventDefault();
    setOver(null);
    const id = e.dataTransfer.getData(DRAG_TYPE);
    const item = id ? useItemsStore.getState().items[id] : undefined;
    if (item && item.status !== status) void useItemsStore.getState().setStatus(id, status);
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
                draggable
                onDragStart={(e) => {
                  e.dataTransfer.setData(DRAG_TYPE, item.id);
                  e.dataTransfer.effectAllowed = "move";
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
                  selected={item.id === selectedId}
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
                Add
              </button>
            )}
          </div>
        </section>
      ))}
    </div>
  );
}
