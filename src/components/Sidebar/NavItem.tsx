import { useState, type MouseEvent, type ReactNode } from "react";
import { moveCards } from "../../stores/bulk";
import { dropAttrs, draggedIds, isCardDrag, type DropPlace } from "./dropTarget";
import styles from "./Sidebar.module.css";

interface NavItemProps {
  label: string;
  icon?: ReactNode;
  /** Project color dot instead of an icon. */
  dot?: string;
  count?: number;
  active?: boolean;
  collapsed?: boolean;
  /** Cards dropped here move to this place. */
  drop?: DropPlace;
  onSelect: () => void;
  /** Right-click menu (projects and boards). */
  onContextMenu?: (e: MouseEvent) => void;
}

export function NavItem({
  label,
  icon,
  dot,
  count,
  active,
  collapsed,
  drop,
  onSelect,
  onContextMenu,
}: NavItemProps) {
  const [over, setOver] = useState(false);
  return (
    <a
      href="#"
      className={styles.nav}
      aria-current={active ? "page" : undefined}
      aria-label={collapsed ? label : undefined}
      title={collapsed ? label : undefined}
      data-drop-over={over || undefined}
      {...(drop ? dropAttrs(drop) : {})}
      onDragOver={
        drop
          ? (e) => {
              if (!isCardDrag(e)) return;
              e.preventDefault();
              e.dataTransfer.dropEffect = "move";
              setOver(true);
            }
          : undefined
      }
      onDragLeave={drop ? () => setOver(false) : undefined}
      onDrop={
        drop
          ? (e) => {
              e.preventDefault();
              setOver(false);
              void moveCards(draggedIds(e), drop.place, drop.name);
            }
          : undefined
      }
      onClick={(e) => {
        e.preventDefault();
        onSelect();
      }}
      onContextMenu={
        onContextMenu &&
        ((e) => {
          e.preventDefault();
          onContextMenu(e);
        })
      }
    >
      {dot ? (
        <span className={styles.dot} data-dot style={{ background: dot }} aria-hidden="true" />
      ) : (
        <span className={styles.icon} aria-hidden="true">
          {icon}
        </span>
      )}
      {!collapsed && <span className={styles.label}>{label}</span>}
      {!collapsed && count !== undefined && count > 0 && (
        <span className={styles.count}>{count}</span>
      )}
    </a>
  );
}
