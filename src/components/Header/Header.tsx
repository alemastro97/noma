import { Plus, Search } from "lucide-react";
import { copy } from "../../copy";
import { AiStatusChip } from "./AiStatusChip";
import styles from "./Header.module.css";
import { ViewSwitcher } from "./ViewSwitcher";
import { FilterToggle } from "../FilterBar/FilterBar";

interface HeaderProps {
  title: string;
  count: number;
  /** Project color; renders a dot before the title when present. */
  color?: string;
  /** Settings and other non-card pages hide the card count and view switcher. */
  plain?: boolean;
  onCapture?: () => void;
  onSearch?: () => void;
}

export function Header({ title, count, color, plain = false, onCapture, onSearch }: HeaderProps) {
  return (
    <header className={styles.header} data-tauri-drag-region>
      <div className={styles.titleGroup}>
        {color && (
          <span className={styles.titleDot} style={{ background: color }} aria-hidden="true" />
        )}
        <h1 className={styles.title}>{title}</h1>
        {!plain && <span className={styles.meta}>{copy.cards(count)}</span>}
      </div>

      {!plain && <ViewSwitcher />}

      <div className={styles.actions}>
        {!plain && <FilterToggle />}
        <label className={styles.search}>
          <Search size={15} strokeWidth={1.75} aria-hidden="true" />
          <input
            type="search"
            placeholder={copy.search}
            aria-label={copy.search}
            onFocus={onSearch}
            readOnly={!!onSearch}
          />
          <span className={styles.meta}>{copy.searchHint}</span>
        </label>
        <button type="button" className={styles.capture} onClick={onCapture}>
          <Plus size={15} strokeWidth={2} aria-hidden="true" />
          {copy.capture}
        </button>
        <AiStatusChip />
        <button type="button" className={styles.avatar} aria-label={copy.profile}>
          A
        </button>
      </div>
    </header>
  );
}
