import { Archive, Pencil, Trash2 } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import * as cardsApi from "../../api/cards";
import type { Board, Project } from "../../api/types";
import { cardsCopy } from "../../copy";
import { PROJECT_COLORS } from "../../lib/colors";
import { confirmAction } from "../../stores/cards";
import { toastError, useToastsStore } from "../../stores/toasts";
import { useUiStore } from "../../stores/ui";
import sidebar from "../Sidebar/Sidebar.module.css";
import styles from "./Cards.module.css";

export type PlaceRef = { kind: "project"; place: Project } | { kind: "board"; place: Board };

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/** Leaves a place that no longer exists in the sidebar. */
function leaveIfShowing(kind: "project" | "board", id: string) {
  const ui = useUiStore.getState();
  if (ui.target.kind === kind && "id" in ui.target && ui.target.id === id) {
    ui.setTarget({ kind: "inbox" });
  }
}

async function archivePlace(ref: PlaceRef) {
  try {
    if (ref.kind === "project") await cardsApi.archiveProject(ref.place.id, true);
    else await cardsApi.archiveBoard(ref.place.id, true);
    leaveIfShowing(ref.kind, ref.place.id);
    useToastsStore.getState().push({
      message: cardsCopy.placeArchived(ref.place.name),
      action: {
        label: cardsCopy.undo,
        run: () =>
          void (
            ref.kind === "project"
              ? cardsApi.archiveProject(ref.place.id, false)
              : cardsApi.archiveBoard(ref.place.id, false)
          ).catch((e: unknown) => toastError(message(e))),
      },
    });
  } catch (err) {
    toastError(message(err));
  }
}

async function deletePlace(ref: PlaceRef) {
  const ok = await confirmAction({
    title:
      ref.kind === "project"
        ? cardsCopy.deleteProjectTitle(ref.place.name)
        : cardsCopy.deleteBoardTitle(ref.place.name),
    body: ref.kind === "project" ? cardsCopy.deleteProjectBody : cardsCopy.deleteBoardBody,
    confirmLabel: cardsCopy.delete,
    danger: true,
  });
  if (!ok) return;
  try {
    const n =
      ref.kind === "project"
        ? await cardsApi.deleteProject(ref.place.id)
        : await cardsApi.deleteBoard(ref.place.id);
    leaveIfShowing(ref.kind, ref.place.id);
    useToastsStore.getState().push({ message: cardsCopy.placeDeleted(ref.place.name, n) });
  } catch (err) {
    toastError(message(err));
  }
}

function MenuItem({
  icon,
  label,
  danger,
  onSelect,
}: {
  icon: ReactNode;
  label: string;
  danger?: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      role="menuitem"
      className={styles.menuItem}
      data-tone={danger ? "danger" : undefined}
      onClick={onSelect}
    >
      {icon}
      {label}
    </button>
  );
}

/** Context menu for a project or personal board in the sidebar. */
export function PlaceMenu({
  target,
  x,
  y,
  onRename,
  onClose,
}: {
  target: PlaceRef;
  x: number;
  y: number;
  onRename: () => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x, y });

  // Keep the menu on screen.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    setPos({
      x: Math.min(x, window.innerWidth - r.width - 8),
      y: Math.min(y, window.innerHeight - r.height - 8),
    });
  }, [x, y]);

  useEffect(() => {
    ref.current?.querySelector<HTMLButtonElement>("button")?.focus();
    const onDown = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", onDown, true);
    window.addEventListener("keydown", onKey);
    window.addEventListener("blur", onClose);
    return () => {
      window.removeEventListener("mousedown", onDown, true);
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("blur", onClose);
    };
  }, [onClose]);

  const run = (fn: () => void) => () => {
    onClose();
    fn();
  };
  const icon = { size: 14, strokeWidth: 1.75, "aria-hidden": true } as const;

  return (
    <div
      ref={ref}
      className={styles.menu}
      role="menu"
      aria-label={cardsCopy.placeMenu(target.place.name)}
      style={{ left: pos.x, top: pos.y }}
    >
      <MenuItem icon={<Pencil {...icon} />} label={cardsCopy.rename} onSelect={run(onRename)} />
      {target.kind === "project" && (
        <div className={styles.swatches} role="group" aria-label={cardsCopy.color}>
          {PROJECT_COLORS.map((c) => (
            <button
              key={c}
              type="button"
              className={styles.swatch}
              style={{ background: c }}
              aria-label={c}
              aria-pressed={target.place.color.toLowerCase() === c.toLowerCase()}
              onClick={run(
                () =>
                  void cardsApi
                    .setProjectColor(target.place.id, c)
                    .catch((e: unknown) => toastError(message(e))),
              )}
            />
          ))}
        </div>
      )}
      <MenuItem
        icon={<Archive {...icon} />}
        label={cardsCopy.archive}
        onSelect={run(() => void archivePlace(target))}
      />
      <div className={styles.menuSep} />
      <MenuItem
        icon={<Trash2 {...icon} />}
        label={cardsCopy.delete}
        danger
        onSelect={run(() => void deletePlace(target))}
      />
    </div>
  );
}

/** Inline rename field that replaces a sidebar row. */
export function RenameInput({
  initial,
  lead,
  onSubmit,
  onDone,
}: {
  initial: string;
  lead: ReactNode;
  onSubmit: (name: string) => void;
  onDone: () => void;
}) {
  const [name, setName] = useState(initial);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    input.current?.focus();
    input.current?.select();
  }, []);
  const submit = () => {
    onDone();
    onSubmit(name);
  };
  return (
    <div className={sidebar.nav} data-creating>
      {lead}
      <input
        ref={input}
        className={sidebar.createInput}
        aria-label={cardsCopy.rename}
        value={name}
        onChange={(e) => setName(e.target.value)}
        onBlur={submit}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            submit();
          } else if (e.key === "Escape") {
            e.preventDefault();
            onDone();
          }
        }}
      />
    </div>
  );
}
