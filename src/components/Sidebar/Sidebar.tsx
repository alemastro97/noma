import {
  Archive,
  CalendarDays,
  FileText,
  Inbox,
  LayoutGrid,
  Lightbulb,
  PanelLeft,
  Plus,
  SlidersHorizontal,
  Sun,
  Trash2,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useShortcutLabels } from "../../hooks/useShortcuts";
import { cardsCopy, copy, sidebarCopy } from "../../copy";
import { nextProjectColor } from "../../lib/colors";
import { useHotkey } from "../../hooks/useHotkey";
import { useItemsStore } from "../../stores/items";
import { personalBoards, useProjectsStore } from "../../stores/projects";
import { sameTarget, useUiStore, type NavTarget } from "../../stores/ui";
import { NomaOrb } from "../NomaOrb";
import { NSymbol } from "../Logo/NSymbol";
import { Wordmark } from "../Logo/Wordmark";
import { NavItem } from "./NavItem";
import { PlaceMenu, RenameInput, type PlaceRef } from "../Cards/PlaceMenu";
import { renamePlace } from "../../stores/cards";
import styles from "./Sidebar.module.css";

const ICON = { size: 17, strokeWidth: 1.75 } as const;

const MAIN_NAV: {
  target: NavTarget;
  label: string;
  icon: React.ReactNode;
  count: "inbox" | "today" | "upcoming" | "ideas" | "notes";
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

function SectionHeader({
  label,
  addLabel,
  onAdd,
}: {
  label: string;
  addLabel: string;
  onAdd: () => void;
}) {
  return (
    <div className={styles.sectionRow}>
      <span className={styles.section}>{label}</span>
      <button
        type="button"
        className={styles.addBtn}
        aria-label={addLabel}
        title={addLabel}
        onClick={onAdd}
      >
        <Plus size={14} strokeWidth={1.75} aria-hidden="true" />
      </button>
    </div>
  );
}

function InlineCreate({
  placeholder,
  dot,
  icon,
  onSubmit,
  onDone,
}: {
  placeholder: string;
  dot?: string;
  icon?: React.ReactNode;
  onSubmit: (name: string) => Promise<void>;
  onDone: () => void;
}) {
  const [name, setName] = useState("");
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => ref.current?.focus(), []);
  const submit = async () => {
    const value = name.trim();
    onDone();
    if (value) await onSubmit(value);
  };
  return (
    <div className={styles.nav} data-creating>
      {dot ? (
        <span className={styles.dot} style={{ background: dot }} aria-hidden="true" />
      ) : (
        <span className={styles.icon} aria-hidden="true">
          {icon}
        </span>
      )}
      <input
        ref={ref}
        className={styles.createInput}
        placeholder={placeholder}
        aria-label={placeholder}
        value={name}
        onChange={(e) => setName(e.target.value)}
        onBlur={() => void submit()}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            void submit();
          } else if (e.key === "Escape") {
            e.preventDefault();
            setName("");
            onDone();
          }
        }}
      />
    </div>
  );
}

export function Sidebar() {
  const keys = useShortcutLabels();
  const counts = useItemsStore((s) => s.counts);
  const projects = useProjectsStore((s) => s.projects);
  const boards = personalBoards(useProjectsStore((s) => s.boards));
  const target = useUiStore((s) => s.target);
  const collapsed = useUiStore((s) => s.sidebarCollapsed);
  const setTarget = useUiStore((s) => s.setTarget);
  const toggleSidebar = useUiStore((s) => s.toggleSidebar);
  const creating = useUiStore((s) => s.creating);
  const setCreating = useUiStore((s) => s.setCreating);
  const createProject = useProjectsStore((s) => s.createProject);
  const createBoard = useProjectsStore((s) => s.createBoard);

  useHotkey("mod+\\", toggleSidebar);
  const [menu, setMenu] = useState<{ ref: PlaceRef; x: number; y: number } | null>(null);
  const [renaming, setRenaming] = useState<PlaceRef | null>(null);
  const closeMenu = useCallback(() => setMenu(null), []);

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
            count={counts[item.count]}
            active={isActive(item.target)}
            collapsed={collapsed}
            onSelect={() => setTarget(item.target)}
          />
        ))}
      </nav>

      {!collapsed || projects.length > 0 ? (
        <nav className={styles.group} aria-label={copy.nav.projects}>
          {!collapsed && (
            <SectionHeader
              label={copy.nav.projects}
              addLabel={sidebarCopy.newProject}
              onAdd={() => setCreating("project")}
            />
          )}
          {projects.map((p) =>
            renaming?.place.id === p.id && !collapsed ? (
              <RenameInput
                key={p.id}
                initial={p.name}
                lead={
                  <span className={styles.dot} style={{ background: p.color }} aria-hidden="true" />
                }
                onSubmit={(name) => void renamePlace("project", p, name)}
                onDone={() => setRenaming(null)}
              />
            ) : (
              <NavItem
                key={p.id}
                label={p.name}
                dot={p.color}
                count={counts.byProject[p.id]}
                active={isActive({ kind: "project", id: p.id })}
                collapsed={collapsed}
                onSelect={() => setTarget({ kind: "project", id: p.id })}
                onContextMenu={(e) =>
                  setMenu({ ref: { kind: "project", place: p }, x: e.clientX, y: e.clientY })
                }
              />
            ),
          )}
          {creating === "project" && !collapsed && (
            <InlineCreate
              placeholder={sidebarCopy.projectPlaceholder}
              dot={nextProjectColor(projects.length)}
              onSubmit={async (name) => {
                const p = await createProject(name, nextProjectColor(projects.length));
                if (p) setTarget({ kind: "project", id: p.id });
              }}
              onDone={() => setCreating(null)}
            />
          )}
        </nav>
      ) : null}

      {!collapsed || boards.length > 0 ? (
        <nav className={styles.group} aria-label={copy.nav.boards}>
          {!collapsed && (
            <SectionHeader
              label={copy.nav.boards}
              addLabel={sidebarCopy.newBoard}
              onAdd={() => setCreating("board")}
            />
          )}
          {boards.map((b) =>
            renaming?.place.id === b.id && !collapsed ? (
              <RenameInput
                key={b.id}
                initial={b.name}
                lead={
                  <span className={styles.icon} aria-hidden="true">
                    <LayoutGrid {...ICON} />
                  </span>
                }
                onSubmit={(name) => void renamePlace("board", b, name)}
                onDone={() => setRenaming(null)}
              />
            ) : (
              <NavItem
                key={b.id}
                label={b.name}
                icon={<LayoutGrid {...ICON} />}
                active={isActive({ kind: "board", id: b.id })}
                collapsed={collapsed}
                onSelect={() => setTarget({ kind: "board", id: b.id })}
                onContextMenu={(e) =>
                  setMenu({ ref: { kind: "board", place: b }, x: e.clientX, y: e.clientY })
                }
              />
            ),
          )}
          {creating === "board" && !collapsed && (
            <InlineCreate
              placeholder={sidebarCopy.boardPlaceholder}
              icon={<LayoutGrid {...ICON} />}
              onSubmit={async (name) => {
                const b = await createBoard(name);
                if (b) setTarget({ kind: "board", id: b.id });
              }}
              onDone={() => setCreating(null)}
            />
          )}
        </nav>
      ) : null}

      <div className={styles.footer}>
        <div
          className={styles.orbCard}
          title={collapsed ? `${copy.orbHere} · ${copy.orbHint(keys.voice)}` : undefined}
        >
          <NomaOrb size={22} label={copy.orbHere} />
          {!collapsed && (
            <div className={styles.orbText}>
              <span>{copy.orbHere}</span>
              <span className={styles.mono}>{copy.orbHint(keys.voice)}</span>
            </div>
          )}
        </div>
        <NavItem
          label={cardsCopy.nav.archive}
          icon={<Archive {...ICON} />}
          active={isActive({ kind: "archive" })}
          collapsed={collapsed}
          onSelect={() => setTarget({ kind: "archive" })}
        />
        <NavItem
          label={cardsCopy.nav.trash}
          icon={<Trash2 {...ICON} />}
          active={isActive({ kind: "trash" })}
          collapsed={collapsed}
          onSelect={() => setTarget({ kind: "trash" })}
        />
        <NavItem
          label={copy.nav.settings}
          icon={<SlidersHorizontal {...ICON} />}
          active={isActive({ kind: "settings" })}
          collapsed={collapsed}
          onSelect={() => setTarget({ kind: "settings" })}
        />
      </div>
      {menu && (
        <PlaceMenu
          target={menu.ref}
          x={menu.x}
          y={menu.y}
          onRename={() => setRenaming(menu.ref)}
          onClose={closeMenu}
        />
      )}
    </aside>
  );
}
