import { Check, Plus, Search } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { captureText } from "../../api/ai";
import { searchItems } from "../../api/items";
import { presenceOpenItem } from "../../api/presence";
import type { Item } from "../../api/types";
import { desktopPresenceCopy as t } from "../../copy";
import styles from "./panel.module.css";

const SEARCH_LIMIT = 7;
const SEARCH_DEBOUNCE_MS = 150;
const SAVED_MS = 1400;

export interface PanelSearchState {
  query: string;
  setQuery: (q: string) => void;
  clear: () => void;
  items: Item[];
}

/** Full-text search over every card, debounced; results replace Today while typing. */
export function usePanelSearch(): PanelSearchState {
  const [query, setQuery] = useState("");
  const [items, setItems] = useState<Item[]>([]);
  useEffect(() => {
    const q = query.trim();
    if (!q) {
      setItems([]);
      return;
    }
    let alive = true;
    const timer = window.setTimeout(() => {
      void searchItems(q, SEARCH_LIMIT)
        .then((r) => alive && setItems(r))
        .catch(() => alive && setItems([]));
    }, SEARCH_DEBOUNCE_MS);
    return () => {
      alive = false;
      window.clearTimeout(timer);
    };
  }, [query]);
  const clear = useCallback(() => setQuery(""), []);
  return { query, setQuery, clear, items };
}

export function PanelSearch({ search }: { search: PanelSearchState }) {
  return (
    <label className={styles.field}>
      <Search size={14} aria-hidden="true" />
      <input
        type="search"
        aria-label={t.searchLabel}
        placeholder={t.searchPlaceholder}
        value={search.query}
        onChange={(e) => search.setQuery(e.target.value)}
      />
    </label>
  );
}

export function SearchResults({ items }: { items: Item[] }) {
  if (items.length === 0) return <p className={styles.empty}>{t.noResults}</p>;
  return (
    <ul className={styles.list}>
      {items.map((item) => (
        <li key={item.id}>
          <button
            type="button"
            className={`${styles.row} ${item.status === "done" ? styles.done : ""}`}
            onClick={() => void presenceOpenItem(item.id).catch(() => undefined)}
          >
            <span
              className={`${styles.check} ${item.kind === "reminder" ? styles.amber : ""}`}
              aria-hidden="true"
            />
            <span className={styles.label}>{item.title}</span>
          </button>
        </li>
      ))}
    </ul>
  );
}

/** One-line capture: ↵ hands the text to NOMA (Auto mode), like quick capture. */
export function PanelCapture() {
  const [value, setValue] = useState("");
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(timer.current), []);

  const submit = async () => {
    const text = value.trim();
    if (!text) return;
    try {
      await captureText(text, {});
      setValue("");
      setSaved(true);
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => setSaved(false), SAVED_MS);
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <label className={styles.field} title={error ?? undefined}>
      {saved ? (
        <Check size={14} className={styles.ok} aria-hidden="true" />
      ) : (
        <Plus size={14} aria-hidden="true" />
      )}
      <input
        type="text"
        aria-label={t.captureLabel}
        placeholder={saved ? t.captureSaved : t.capturePlaceholder}
        value={value}
        onChange={(e) => {
          setValue(e.target.value);
          setError(null);
          setSaved(false);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.nativeEvent.isComposing) {
            e.preventDefault();
            void submit();
          }
        }}
      />
    </label>
  );
}
