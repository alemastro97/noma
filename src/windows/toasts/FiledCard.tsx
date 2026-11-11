import { Check, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { listBoards } from "../../api/boards";
import { filedMove, filedOpen, filedUndo, reminderDismiss } from "../../api/desktop";
import { listProjects } from "../../api/projects";
import type { Place, ReminderToast } from "../../api/types";
import { desktopCopy } from "../../copy";
import glass from "../shared/glass.module.css";
import { movePlaces, type PlaceOption } from "./places";
import styles from "./toasts.module.css";

const CLOSE_AFTER = 7000;

/** "Filed in …" after a capture: says where the card went; Undo, Move or Open. */
export function FiledCard({ toast, onResize }: { toast: ReminderToast; onResize: () => void }) {
  const filed = toast.filed!;
  const first = toast.items[0];
  const ids = toast.items.map((i) => i.id);
  const [places, setPlaces] = useState<PlaceOption[] | null>(null);
  const [hover, setHover] = useState(false);
  const timer = useRef<number | undefined>(undefined);

  // Goes away by itself, unless the user is on it or choosing a place.
  const busy = hover || places !== null;
  useEffect(() => {
    window.clearTimeout(timer.current);
    if (!busy) {
      timer.current = window.setTimeout(() => void reminderDismiss(toast.id), CLOSE_AFTER);
    }
    return () => window.clearTimeout(timer.current);
  }, [busy, toast.id, filed.placeName]);

  useEffect(onResize, [places, onResize]);

  const openMenu = async () => {
    if (places) return setPlaces(null);
    const [projects, boards] = await Promise.all([listProjects(), listBoards()]);
    setPlaces(movePlaces(projects, boards, filed.place));
  };

  const move = (place: Place) => {
    setPlaces(null);
    void filedMove(filed.captureId, ids, place);
  };

  if (!first) return null;
  return (
    <section
      className={`${glass.glass} ${styles.toast}`}
      aria-label="Capture filed"
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
    >
      <div className={`${styles.icon} ${styles.filedIcon}`} aria-hidden="true">
        <Check size={20} strokeWidth={1.75} />
      </div>
      <div className={styles.body}>
        <div className={styles.text}>
          <span className={styles.title}>{first.title}</span>
          <span className={styles.place} aria-live="polite">
            {filed.moved
              ? desktopCopy.movedTo(filed.placeName)
              : desktopCopy.filedIn(filed.placeName)}
            {toast.items.length > 1 && ` · ${desktopCopy.andMore(toast.items.length - 1)}`}
          </span>
        </div>
        {places ? (
          <div className={styles.places} role="menu" aria-label={desktopCopy.moveTo}>
            {places.map((p) => (
              <button
                key={p.key}
                type="button"
                role="menuitem"
                className={`${glass.btn} ${styles.placeBtn}`}
                onClick={() => move(p.place)}
              >
                {p.color && <span className={styles.swatch} style={{ background: p.color }} />}
                {p.name}
              </button>
            ))}
          </div>
        ) : (
          <div className={styles.actions}>
            {!filed.moved && (
              <button
                type="button"
                className={glass.btn}
                onClick={() => void filedUndo(filed.captureId, ids)}
              >
                {desktopCopy.undo}
              </button>
            )}
            <button type="button" className={glass.btn} onClick={() => void openMenu()}>
              {desktopCopy.move}
            </button>
            <button
              type="button"
              className={`${glass.btn} ${glass.btnDone}`}
              onClick={() => void filedOpen(toast.id, first.id)}
            >
              {desktopCopy.open}
            </button>
          </div>
        )}
      </div>
      <button
        type="button"
        className={styles.close}
        aria-label={desktopCopy.dismiss}
        onClick={() => void reminderDismiss(toast.id)}
      >
        <X size={14} />
      </button>
    </section>
  );
}
