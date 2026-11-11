import { ArrowUpRight, Mic } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { mainShow, panelHide, voiceStart } from "../../api/desktop";
import { onItemsChanged } from "../../api/events";
import { completeItem, listItems, updateItem } from "../../api/items";
import type { Item } from "../../api/types";
import { desktopCopy } from "../../copy";
import { PanelCapture, PanelSearch, SearchResults, usePanelSearch } from "./PanelFields";
import { anchorDate, formatTime } from "../../lib/dates";
import glass from "../shared/glass.module.css";
import { todayRows } from "./today";
import styles from "./panel.module.css";
import { useFocus } from "../../hooks/useFocus";
import { FocusBar, FocusButton, focusRowClass } from "./FocusBar";

const MAX_ROWS = 7;

/** The orb's panel: today at a glance, tick things off, talk, or open NOMA. */
export function PanelWindow() {
  const [items, setItems] = useState<Item[]>([]);
  const focus = useFocus();
  const search = usePanelSearch();
  const queryRef = useRef(search.query);
  useEffect(() => {
    queryRef.current = search.query;
  }, [search.query]);
  const clearSearch = search.clear;

  const load = useCallback(() => {
    void listItems({ kind: "today" }, true)
      .then(setItems)
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    load();
    const un = onItemsChanged(load);
    window.addEventListener("focus", load);
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      // Esc first clears the search, then closes the panel.
      if (queryRef.current) clearSearch();
      else void panelHide("panel");
    };
    window.addEventListener("keydown", onKey);
    return () => {
      void un.then((f) => f());
      window.removeEventListener("focus", load);
      window.removeEventListener("keydown", onKey);
    };
  }, [load, clearSearch]);

  const toggle = (item: Item) => {
    const done = item.status === "done";
    setItems((prev) =>
      prev.map((i) => (i.id === item.id ? { ...i, status: done ? "next" : "done" } : i)),
    );
    void (done ? updateItem(item.id, { status: "next" }) : completeItem(item.id)).catch(load);
  };

  const { rows, left } = todayRows(items, MAX_ROWS);

  return (
    <div className={styles.root}>
      <section className={`${glass.glass} ${styles.panel}`} aria-label="NOMA panel">
        <header className={styles.head}>
          <span className={styles.title}>{desktopCopy.today}</span>
          <span className={styles.meta}>{desktopCopy.left(left)}</span>
        </header>
        <FocusBar clock={focus} />

        <PanelSearch search={search} />

        {search.query.trim() ? (
          <SearchResults items={search.items} />
        ) : rows.length === 0 ? (
          <p className={styles.empty}>{desktopCopy.allClear}</p>
        ) : (
          <ul className={styles.list}>
            {rows.map((item) => {
              const done = item.status === "done";
              const at = anchorDate(item);
              const timed = at && (at.getHours() !== 0 || at.getMinutes() !== 0);
              return (
                <li key={item.id} className={focusRowClass}>
                  <button
                    type="button"
                    className={`${styles.row} ${done ? styles.done : ""}`}
                    onClick={() => toggle(item)}
                    aria-pressed={done}
                  >
                    <span
                      className={`${styles.check} ${item.kind === "reminder" ? styles.amber : ""}`}
                      aria-hidden="true"
                    />
                    <span className={styles.label}>{item.title}</span>
                    {timed && !done && <span className={styles.time}>{formatTime(at)}</span>}
                  </button>
                  {!done && <FocusButton title={item.title} onStart={() => focus.start(item.id)} />}
                </li>
              );
            })}
          </ul>
        )}

        <PanelCapture />

        <footer className={styles.foot}>
          <button
            type="button"
            className={glass.btn}
            onClick={() => void voiceStart()}
            aria-label={desktopCopy.talk}
          >
            <Mic size={14} aria-hidden="true" /> {desktopCopy.talk}
          </button>
          <button
            type="button"
            className={`${glass.btn} ${styles.open}`}
            onClick={() => void mainShow()}
          >
            {desktopCopy.openNoma} <ArrowUpRight size={14} aria-hidden="true" />
          </button>
        </footer>
      </section>
    </div>
  );
}
