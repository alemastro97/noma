import {
  CalendarDays,
  FileText,
  Inbox,
  LayoutGrid,
  Lightbulb,
  PanelLeft,
  SlidersHorizontal,
  Sun,
} from "lucide-react";
import { copy } from "../../copy";
import { useHotkey } from "../../hooks/useHotkey";
import { sameTarget, useUiStore, type NavTarget } from "../../stores/ui";
import { NomaOrb } from "../NomaOrb";
import { NSymbol } from "../Logo/NSymbol";
import { Wordmark } from "../Logo/Wordmark";
import { NavItem } from "./NavItem";
import styles from "./Sidebar.module.css";

export interface SidebarCounts {
  inbox: number;
  today: number;
  upcoming: number;
  ideas: number;
  notes: number;
  byProject: Record<string, number>;
}

export interface SidebarProject {
  id: string;
  name: string;
  color: string;
}

export interface SidebarBoard {
  id: string;
  name: string;
}

interface SidebarProps {
  counts: SidebarCounts;
  projects: SidebarProject[];
  boards: SidebarBoard[];
}

const ICON = { size: 17, strokeWidth: 1.75 } as const;

const MAIN_NAV: {
  target: NavTarget;
  label: string;
  icon: React.ReactNode;
  count: keyof SidebarCounts;
}[] = [
  { target: { kind: "inbox" }, label: copy.nav.inbox, icon: <Inbox {...ICON} />, count: "inbox" },
  { target: { kind: "today" }, label: copy.nav.today, icon: <Sun {...ICON} />, count: "today" },
  {
    target: { kind: "upcoming" },
    label: copy.nav.upcoming,
    icon: <CalendarDays {...ICON} />,
    count: "upcoming",
  },
  {
    target: { kind: "ideas" },
    label: copy.nav.ideas,
    icon: <Lightbulb {...ICON} />,
    count: "ideas",
  },
  {
    target: { kind: "notes" },
    label: copy.nav.notes,
    icon: <FileText {...ICON} />,
    count: "notes",
  },
];

export function Sidebar({ counts, projects, boards }: SidebarProps) {
  const target = useUiStore((s) => s.target);
  const collapsed = useUiStore((s) => s.sidebarCollapsed);
  const setTarget = useUiStore((s) => s.setTarget);
  const toggleSidebar = useUiStore((s) => s.toggleSidebar);

  useHotkey("mod+\\", toggleSidebar);

  const isActive = (t: NavTarget) => sameTarget(t, target);

  return (
    <aside className={styles.sidebar} data-collapsed={collapsed ? "true" : "false"}>
      <div className={styles.titlebarSpacer} data-tauri-drag-region />
      <div className={styles.brand} data-tauri-drag-region>
        <NSymbol size={22} className={styles.symbol} />
        {!collapsed && <Wordmark />}
        <button
          type="button"
          className={styles.collapseBtn}
          aria-label={collapsed ? copy.nav.expand : copy.nav.collapse}
          title={`${collapsed ? copy.nav.expand : copy.nav.collapse} (⌘\\)`}
          onClick={toggleSidebar}
        >
          <PanelLeft size={16} strokeWidth={1.75} />
        </button>
      </div>

      <nav className={styles.group} aria-label="Main">
        {MAIN_NAV.map((item) => (
          <NavItem
            key={item.label}
            label={item.label}
            icon={item.icon}
            count={counts[item.count] as number}
            active={isActive(item.target)}
            collapsed={collapsed}
            onSelect={() => setTarget(item.target)}
          />
        ))}
      </nav>

      {projects.length > 0 && (
        <nav className={styles.group} aria-label={copy.nav.projects}>
          {!collapsed && <span className={styles.section}>{copy.nav.projects}</span>}
          {projects.map((p) => (
            <NavItem
              key={p.id}
              label={p.name}
              dot={p.color}
              count={counts.byProject[p.id]}
              active={isActive({ kind: "project", id: p.id })}
              collapsed={collapsed}
              onSelect={() => setTarget({ kind: "project", id: p.id })}
            />
          ))}
        </nav>
      )}

      {boards.length > 0 && (
        <nav className={styles.group} aria-label={copy.nav.boards}>
          {!collapsed && <span className={styles.section}>{copy.nav.boards}</span>}
          {boards.map((b) => (
            <NavItem
              key={b.id}
              label={b.name}
              icon={<LayoutGrid {...ICON} />}
              active={isActive({ kind: "board", id: b.id })}
              collapsed={collapsed}
              onSelect={() => setTarget({ kind: "board", id: b.id })}
            />
          ))}
        </nav>
      )}

      <div className={styles.footer}>
        <div
          className={styles.orbCard}
          title={collapsed ? `${copy.orbHere} · ${copy.orbHint}` : undefined}
        >
          <NomaOrb size={22} label={copy.orbHere} />
          {!collapsed && (
            <div className={styles.orbText}>
              <span>{copy.orbHere}</span>
              <span className={styles.mono}>{copy.orbHint}</span>
            </div>
          )}
        </div>
        <NavItem
          label={copy.nav.settings}
          icon={<SlidersHorizontal {...ICON} />}
          active={isActive({ kind: "settings" })}
          collapsed={collapsed}
          onSelect={() => setTarget({ kind: "settings" })}
        />
      </div>
    </aside>
  );
}
