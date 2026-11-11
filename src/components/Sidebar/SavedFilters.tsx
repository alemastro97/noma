import { ListFilter, X } from "lucide-react";
import { workspaceCopy } from "../../copy";
import { useFiltersStore } from "../../stores/filters";
import { sameTarget, useUiStore } from "../../stores/ui";
import { NavItem } from "./NavItem";
import styles from "./Sidebar.module.css";
import own from "./SavedFilters.module.css";

/** Saved filters as sidebar entries (hidden until one is saved). */
export function SavedFilters({ collapsed }: { collapsed: boolean }) {
  const saved = useFiltersStore((s) => s.saved);
  const remove = useFiltersStore((s) => s.remove);
  const target = useUiStore((s) => s.target);
  const setTarget = useUiStore((s) => s.setTarget);
  if (saved.length === 0) return null;

  return (
    <nav className={styles.group} aria-label={workspaceCopy.filters}>
      {!collapsed && <span className={styles.section}>{workspaceCopy.filters}</span>}
      {saved.map((f) => (
        <div key={f.id} className={own.row}>
          <NavItem
            label={f.name}
            icon={<ListFilter size={17} strokeWidth={1.75} />}
            active={sameTarget(target, { kind: "filter", id: f.id })}
            collapsed={collapsed}
            onSelect={() => setTarget({ kind: "filter", id: f.id })}
          />
          {!collapsed && (
            <button
              type="button"
              className={own.remove}
              aria-label={workspaceCopy.deleteFilter(f.name)}
              title={workspaceCopy.deleteFilter(f.name)}
              onClick={() => void remove(f.id)}
            >
              <X size={13} strokeWidth={1.75} aria-hidden="true" />
            </button>
          )}
        </div>
      ))}
    </nav>
  );
}
