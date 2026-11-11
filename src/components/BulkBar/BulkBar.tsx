import { CalendarDays, Check, FolderInput, Inbox, LayoutGrid, Trash2, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { Place } from "../../api/types";
import { workspaceCopy } from "../../copy";
import { bulkComplete, bulkDelete, bulkSetDate, moveCards } from "../../stores/bulk";
import { personalBoards, useProjectsStore } from "../../stores/projects";
import { useSelectionStore } from "../../stores/selection";
import { useUiStore } from "../../stores/ui";
import styles from "./BulkBar.module.css";

type Menu = "move" | "date" | null;

/** 09:00 local on a YYYY-MM-DD day, as an ISO timestamp. */
function dayToIso(day: string): string | null {
  const [y, m, d] = day.split("-").map(Number);
  if (!y || !m || !d) return null;
  return new Date(y, m - 1, d, 9, 0, 0, 0).toISOString();
}

/** Floating actions for two or more selected cards. */
export function BulkBar() {
  const ids = useSelectionStore((s) => s.ids);
  const target = useUiStore((s) => s.target);
  const projects = useProjectsStore((s) => s.projects);
  const boards = personalBoards(useProjectsStore((s) => s.boards));
  const [menu, setMenu] = useState<Menu>(null);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!menu) return;
    const onDown = (e: PointerEvent) => {
      if (!ref.current?.contains(e.target as Node)) setMenu(null);
    };
    window.addEventListener("pointerdown", onDown);
    return () => window.removeEventListener("pointerdown", onDown);
  }, [menu]);

  if (ids.length < 2 || target.kind === "settings") return null;

  const clear = () => {
    useSelectionStore.getState().clear();
    useUiStore.getState().select(null);
  };
  const move = (place: Place, name: string) => {
    setMenu(null);
    void moveCards(ids, place, name);
  };

  return (
    <div
      ref={ref}
      className={styles.bar}
      role="toolbar"
      aria-label={workspaceCopy.selected(ids.length)}
    >
      <span className={styles.count}>{workspaceCopy.selected(ids.length)}</span>
      <span className={styles.sep} aria-hidden="true" />
      <button type="button" className={styles.btn} onClick={() => bulkComplete(ids)}>
        <Check size={14} strokeWidth={2} aria-hidden="true" />
        {workspaceCopy.bulkDone}
      </button>
      <div className={styles.anchor}>
        <button
          type="button"
          className={styles.btn}
          aria-expanded={menu === "move"}
          onClick={() => setMenu(menu === "move" ? null : "move")}
        >
          <FolderInput size={14} strokeWidth={1.75} aria-hidden="true" />
          {workspaceCopy.bulkMove}
        </button>
        {menu === "move" && (
          <div className={styles.menu} role="menu" aria-label={workspaceCopy.moveTo}>
            <span className={styles.menuLabel}>{workspaceCopy.moveTo}</span>
            <button
              type="button"
              role="menuitem"
              className={styles.item}
              onClick={() => move({ kind: "inbox" }, workspaceCopy.inbox)}
            >
              <Inbox size={14} strokeWidth={1.75} aria-hidden="true" />
              {workspaceCopy.inbox}
            </button>
            {projects.map((p) => (
              <button
                key={p.id}
                type="button"
                role="menuitem"
                className={styles.item}
                onClick={() => move({ kind: "project", id: p.id }, p.name)}
              >
                <span className={styles.dot} style={{ background: p.color }} aria-hidden="true" />
                {p.name}
              </button>
            ))}
            {boards.map((b) => (
              <button
                key={b.id}
                type="button"
                role="menuitem"
                className={styles.item}
                onClick={() => move({ kind: "board", id: b.id }, b.name)}
              >
                <LayoutGrid size={14} strokeWidth={1.75} aria-hidden="true" />
                {b.name}
              </button>
            ))}
          </div>
        )}
      </div>
      <div className={styles.anchor}>
        <button
          type="button"
          className={styles.btn}
          aria-expanded={menu === "date"}
          onClick={() => setMenu(menu === "date" ? null : "date")}
        >
          <CalendarDays size={14} strokeWidth={1.75} aria-hidden="true" />
          {workspaceCopy.bulkDate}
        </button>
        {menu === "date" && (
          <div className={styles.menu} role="dialog" aria-label={workspaceCopy.setDate}>
            <span className={styles.menuLabel}>{workspaceCopy.setDate}</span>
            <input
              type="date"
              className={styles.dateInput}
              aria-label={workspaceCopy.setDate}
              onChange={(e) => {
                const iso = dayToIso(e.target.value);
                if (!iso) return;
                bulkSetDate(ids, iso);
                setMenu(null);
              }}
            />
            <button
              type="button"
              className={styles.item}
              onClick={() => {
                bulkSetDate(ids, null);
                setMenu(null);
              }}
            >
              {workspaceCopy.clearDate}
            </button>
          </div>
        )}
      </div>
      <button
        type="button"
        className={styles.btn}
        data-tone="danger"
        onClick={() => bulkDelete(ids)}
      >
        <Trash2 size={14} strokeWidth={1.75} aria-hidden="true" />
        {workspaceCopy.bulkDelete}
      </button>
      <span className={styles.sep} aria-hidden="true" />
      <button
        type="button"
        className={styles.close}
        aria-label={workspaceCopy.bulkClear}
        title={workspaceCopy.bulkClear}
        onClick={clear}
      >
        <X size={14} strokeWidth={1.75} aria-hidden="true" />
      </button>
    </div>
  );
}
