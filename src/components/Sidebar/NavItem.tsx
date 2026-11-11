import type { MouseEvent, ReactNode } from "react";
import styles from "./Sidebar.module.css";

interface NavItemProps {
  label: string;
  icon?: ReactNode;
  /** Project color dot instead of an icon. */
  dot?: string;
  count?: number;
  active?: boolean;
  collapsed?: boolean;
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
  onSelect,
  onContextMenu,
}: NavItemProps) {
  return (
    <a
      href="#"
      className={styles.nav}
      aria-current={active ? "page" : undefined}
      aria-label={collapsed ? label : undefined}
      title={collapsed ? label : undefined}
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
