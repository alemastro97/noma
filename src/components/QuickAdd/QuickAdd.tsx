import { useEffect, useRef, useState } from "react";
import type { ItemType, NewItem } from "../../api/types";
import { kindLabel, quickAddCopy } from "../../copy";
import { useItemsStore } from "../../stores/items";
import { useProjectsStore } from "../../stores/projects";
import { useUiStore } from "../../stores/ui";
import { KIND_STYLE } from "../Card/kind";
import styles from "./Overlay.module.css";

const KINDS: ItemType[] = ["task", "reminder", "note", "idea", "event"];

/** Manual capture: a title and a type. AI parsing replaces this in Phase 2. */
export function QuickAdd() {
  const open = useUiStore((s) => s.quickAdd.open);
  // Mounting the dialog only while open gives it fresh state every time.
  return open ? <QuickAddDialog /> : null;
}

function QuickAddDialog() {
  const quickAdd = useUiStore((s) => s.quickAdd);
  const target = useUiStore((s) => s.target);
  const close = useUiStore((s) => s.closeQuickAdd);
  const projects = useProjectsStore((s) => s.projects);
  const [title, setTitle] = useState("");
  const [kind, setKind] = useState<ItemType>(
    () =>
      quickAdd.kind ??
      (target.kind === "ideas" ? "idea" : target.kind === "notes" ? "note" : "task"),
  );
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    requestAnimationFrame(() => inputRef.current?.focus());
  }, []);

  const project = target.kind === "project" ? projects.find((p) => p.id === target.id) : undefined;

  const submit = async () => {
    const text = title.trim();
    if (!text) return;
    const input: NewItem = {
      kind,
      title: text,
      status: quickAdd.status ?? null,
      projectId: project?.id ?? null,
      boardId: target.kind === "board" ? target.id : (project?.boardId ?? null),
      x: quickAdd.position?.x ?? null,
      y: quickAdd.position?.y ?? null,
    };
    if (target.kind === "today" && (kind === "task" || kind === "event")) {
      const end = new Date();
      end.setHours(18, 0, 0, 0);
      input.dueAt = end.toISOString();
    }
    close();
    const created = await useItemsStore.getState().create(input);
    if (created) useUiStore.getState().select(created.id);
  };

  return (
    <div className={styles.backdrop} onMouseDown={close}>
      <div
        className={styles.panel}
        role="dialog"
        aria-label="Quick add"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <input
          ref={inputRef}
          className={styles.input}
          placeholder={quickAddCopy.placeholder}
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              void submit();
            } else if (e.key === "Escape") {
              e.preventDefault();
              close();
            } else if (e.key === "Tab") {
              e.preventDefault();
              const i = KINDS.indexOf(kind);
              setKind(KINDS[(i + (e.shiftKey ? KINDS.length - 1 : 1)) % KINDS.length]!);
            }
          }}
        />
        <div className={styles.footer}>
          <div className={styles.kinds} role="radiogroup" aria-label="Type">
            {KINDS.map((k) => {
              const { Icon, color } = KIND_STYLE[k];
              const active = k === kind;
              return (
                <button
                  key={k}
                  type="button"
                  role="radio"
                  aria-checked={active}
                  className={styles.kind}
                  style={active ? { color, borderColor: color } : undefined}
                  onClick={() => {
                    setKind(k);
                    inputRef.current?.focus();
                  }}
                >
                  <Icon size={13} strokeWidth={1.75} aria-hidden={true} />
                  {kindLabel[k]}
                </button>
              );
            })}
          </div>
          <span className={styles.hint}>
            {project ? `${project.name} · ` : ""}
            {quickAddCopy.hint}
          </span>
        </div>
      </div>
    </div>
  );
}
