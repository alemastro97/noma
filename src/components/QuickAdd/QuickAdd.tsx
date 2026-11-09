import { Sparkles } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { CaptureContext, ItemType, NewItem } from "../../api/types";
import { kindLabel, quickAddCopy } from "../../copy";
import { useAiStore } from "../../stores/ai";
import { useItemsStore } from "../../stores/items";
import { useProjectsStore } from "../../stores/projects";
import { useUiStore } from "../../stores/ui";
import { KIND_STYLE } from "../Card/kind";
import styles from "./Overlay.module.css";

/** "auto" lets the on-device model decide the type, dates and project. */
type Mode = "auto" | ItemType;
const MODES: Mode[] = ["auto", "task", "reminder", "note", "idea", "event"];

/** Quick capture: free text sorted by AI ("Auto"), or a title with a chosen type. */
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
  const ai = useAiStore((s) => s.state);
  const [title, setTitle] = useState("");
  const [mode, setMode] = useState<Mode>(() => quickAdd.kind ?? "auto");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    requestAnimationFrame(() => inputRef.current?.focus());
  }, []);

  const project = target.kind === "project" ? projects.find((p) => p.id === target.id) : undefined;
  const boardId = target.kind === "board" ? target.id : (project?.boardId ?? null);

  const submit = async () => {
    const text = title.trim();
    if (!text) return;
    close();

    if (mode === "auto") {
      const context: CaptureContext = {
        projectId: project?.id ?? null,
        boardId,
        x: quickAdd.position?.x ?? null,
        y: quickAdd.position?.y ?? null,
        status: quickAdd.status ?? null,
      };
      const item = await useAiStore.getState().capture(text, context);
      if (item) useUiStore.getState().select(item.id);
      return;
    }

    const input: NewItem = {
      kind: mode,
      title: text,
      status: quickAdd.status ?? null,
      projectId: project?.id ?? null,
      boardId,
      x: quickAdd.position?.x ?? null,
      y: quickAdd.position?.y ?? null,
    };
    if (target.kind === "today" && (mode === "task" || mode === "event")) {
      const end = new Date();
      end.setHours(18, 0, 0, 0);
      input.dueAt = end.toISOString();
    }
    const created = await useItemsStore.getState().create(input);
    if (created) useUiStore.getState().select(created.id);
  };

  const hint =
    mode === "auto"
      ? ai?.ready
        ? quickAddCopy.autoReady
        : quickAddCopy.autoPending
      : quickAddCopy.hint;

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
          placeholder={mode === "auto" ? quickAddCopy.autoPlaceholder : quickAddCopy.placeholder}
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
              const i = MODES.indexOf(mode);
              setMode(MODES[(i + (e.shiftKey ? MODES.length - 1 : 1)) % MODES.length]!);
            }
          }}
        />
        <div className={styles.footer}>
          <div className={styles.kinds} role="radiogroup" aria-label="Type">
            {MODES.map((m) => {
              const active = m === mode;
              const { Icon, color } =
                m === "auto" ? { Icon: Sparkles, color: "var(--noma-lavender)" } : KIND_STYLE[m];
              return (
                <button
                  key={m}
                  type="button"
                  role="radio"
                  aria-checked={active}
                  className={styles.kind}
                  style={active ? { color, borderColor: color } : undefined}
                  onClick={() => {
                    setMode(m);
                    inputRef.current?.focus();
                  }}
                >
                  <Icon size={13} strokeWidth={1.75} aria-hidden={true} />
                  {m === "auto" ? quickAddCopy.auto : kindLabel[m]}
                </button>
              );
            })}
          </div>
          <span className={styles.hint}>
            {project ? `${project.name} · ` : ""}
            {hint}
          </span>
        </div>
      </div>
    </div>
  );
}
