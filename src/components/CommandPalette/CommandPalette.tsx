import {
  ArrowRight,
  Columns3,
  FolderPlus,
  LayoutGrid,
  List,
  Moon,
  PanelLeft,
  Plus,
  Search,
  Sparkles,
  Spline,
  Sun,
  SunMoon,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { devSeed, searchItems } from "../../api/items";
import type { Item, ItemType } from "../../api/types";
import { copy, kindLabel, paletteCopy, sidebarCopy } from "../../copy";
import { useItemsStore } from "../../stores/items";
import { useProjectsStore, personalBoards } from "../../stores/projects";
import { useToastsStore } from "../../stores/toasts";
import { useUiStore, type NavTarget } from "../../stores/ui";
import { setTheme } from "../../theme/theme";
import { KIND_STYLE } from "../Card/kind";
import styles from "../QuickAdd/Overlay.module.css";
import { filterCommands, type Command } from "./commands";

type Option = { type: "command"; cmd: Command; icon: ReactNode } | { type: "item"; item: Item };

const NEW_KINDS: ItemType[] = ["task", "reminder", "note", "idea", "event"];

function useCommands(): (Command & { icon: ReactNode })[] {
  const projects = useProjectsStore((s) => s.projects);
  const boards = useProjectsStore((s) => s.boards);
  return useMemo(() => {
    const ui = useUiStore.getState;
    const go = (target: NavTarget) => () => ui().setTarget(target);
    const icon = <ArrowRight size={15} strokeWidth={1.75} />;
    const list: (Command & { icon: ReactNode })[] = [
      ...(["inbox", "today", "upcoming", "ideas", "notes"] as const).map((k) => ({
        id: `go:${k}`,
        label: `Go to ${copy.nav[k]}`,
        section: "goTo" as const,
        run: go({ kind: k }),
        icon,
      })),
      ...projects.map((p) => ({
        id: `go:project:${p.id}`,
        label: p.name,
        keywords: "project go",
        section: "goTo" as const,
        run: go({ kind: "project", id: p.id }),
        icon: (
          <span
            style={{
              width: 8,
              height: 8,
              borderRadius: "50%",
              background: p.color,
              margin: "0 3px",
            }}
          />
        ),
      })),
      ...personalBoards(boards).map((b) => ({
        id: `go:board:${b.id}`,
        label: b.name,
        keywords: "board go",
        section: "goTo" as const,
        run: go({ kind: "board", id: b.id }),
        icon: <LayoutGrid size={15} strokeWidth={1.75} />,
      })),
      ...NEW_KINDS.map((k) => {
        const { Icon } = KIND_STYLE[k];
        return {
          id: `new:${k}`,
          label: `New ${kindLabel[k].toLowerCase()}`,
          keywords: "create add capture",
          hint: k === "task" ? "⌘N" : undefined,
          section: "actions" as const,
          run: () => ui().openQuickAdd({ kind: k }),
          icon: <Icon size={15} strokeWidth={1.75} />,
        };
      }),
      {
        id: "new:project",
        label: sidebarCopy.newProject,
        keywords: "create",
        section: "actions",
        run: () => ui().setCreating("project"),
        icon: <FolderPlus size={15} strokeWidth={1.75} />,
      },
      {
        id: "new:board",
        label: sidebarCopy.newBoard,
        keywords: "create canvas",
        section: "actions",
        run: () => ui().setCreating("board"),
        icon: <Plus size={15} strokeWidth={1.75} />,
      },
      {
        id: "view:canvas",
        label: "Canvas view",
        keywords: "switch mode",
        hint: "⌘1",
        section: "actions",
        run: () => ui().setViewMode("canvas"),
        icon: <Spline size={15} strokeWidth={1.75} />,
      },
      {
        id: "view:kanban",
        label: "Kanban view",
        keywords: "switch mode board columns",
        hint: "⌘2",
        section: "actions",
        run: () => ui().setViewMode("kanban"),
        icon: <Columns3 size={15} strokeWidth={1.75} />,
      },
      {
        id: "view:list",
        label: "List view",
        keywords: "switch mode",
        hint: "⌘3",
        section: "actions",
        run: () => ui().setViewMode("list"),
        icon: <List size={15} strokeWidth={1.75} />,
      },
      {
        id: "sidebar",
        label: "Toggle sidebar",
        hint: "⌘\\",
        section: "actions",
        run: () => ui().toggleSidebar(),
        icon: <PanelLeft size={15} strokeWidth={1.75} />,
      },
      {
        id: "theme:dark",
        label: "Theme: Dark",
        keywords: "appearance",
        section: "actions",
        run: () => setTheme("dark"),
        icon: <Moon size={15} strokeWidth={1.75} />,
      },
      {
        id: "theme:light",
        label: "Theme: Light",
        keywords: "appearance",
        section: "actions",
        run: () => setTheme("light"),
        icon: <Sun size={15} strokeWidth={1.75} />,
      },
      {
        id: "theme:system",
        label: "Theme: Match system",
        keywords: "appearance auto",
        section: "actions",
        run: () => setTheme("system"),
        icon: <SunMoon size={15} strokeWidth={1.75} />,
      },
    ];
    if (import.meta.env.DEV) {
      list.push({
        id: "dev:seed",
        label: "Load sample data",
        keywords: "dev seed demo",
        section: "actions",
        run: () =>
          void devSeed().then((seeded) => {
            void useProjectsStore.getState().load();
            useToastsStore.getState().push({
              message: seeded ? "Sample data loaded." : "Sample data needs an empty database.",
            });
          }),
        icon: <Sparkles size={15} strokeWidth={1.75} />,
      });
    }
    return list;
  }, [projects, boards]);
}

export function CommandPalette() {
  const open = useUiStore((s) => s.paletteOpen);
  // Mounting the dialog only while open gives it fresh state every time.
  return open ? <PaletteDialog /> : null;
}

function PaletteDialog() {
  const setOpen = useUiStore((s) => s.setPaletteOpen);
  const commands = useCommands();
  const [query, setQuery] = useState("");
  const [search, setSearch] = useState<{ q: string; items: Item[] }>({ q: "", items: [] });
  const [active, setActive] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const q = query.trim();
  const results = useMemo(() => (q.length >= 2 && search.q === q ? search.items : []), [q, search]);

  useEffect(() => {
    requestAnimationFrame(() => inputRef.current?.focus());
  }, []);

  // Debounced full-text search.
  useEffect(() => {
    if (q.length < 2) return;
    let alive = true;
    const t = setTimeout(() => {
      void searchItems(q, 8).then((r) => {
        if (alive) setSearch({ q, items: r ?? [] });
      });
    }, 120);
    return () => {
      alive = false;
      clearTimeout(t);
    };
  }, [q]);

  const options: Option[] = useMemo(() => {
    const cmds = filterCommands(commands, query) as (Command & { icon: ReactNode })[];
    const shown = query.trim() ? cmds.slice(0, 8) : cmds;
    return [
      ...results.map((item) => ({ type: "item" as const, item })),
      ...shown.map((cmd) => ({ type: "command" as const, cmd, icon: cmd.icon })),
    ];
  }, [commands, query, results]);

  const activeIndex = Math.min(active, Math.max(options.length - 1, 0));
  useEffect(() => {
    listRef.current
      ?.querySelector<HTMLElement>(`[data-index="${activeIndex}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [activeIndex]);

  const close = () => setOpen(false);
  const run = (opt: Option | undefined) => {
    close();
    if (!opt) {
      const title = query.trim();
      if (title) void useItemsStore.getState().create({ kind: "task", title });
      return;
    }
    if (opt.type === "command") opt.cmd.run();
    else useUiStore.getState().openDetail(opt.item.id);
  };

  let lastSection: string | null = null;

  return (
    <div className={styles.backdrop} onMouseDown={close}>
      <div
        className={styles.panel}
        role="dialog"
        aria-label="Command palette"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            paddingLeft: 18,
            color: "var(--noma-text-2)",
          }}
        >
          <Search size={17} strokeWidth={1.75} aria-hidden="true" />
          <input
            ref={inputRef}
            className={styles.input}
            style={{ paddingLeft: 12 }}
            placeholder={paletteCopy.placeholder}
            value={query}
            role="combobox"
            aria-expanded="true"
            aria-controls="palette-list"
            onChange={(e) => {
              setQuery(e.target.value);
              setActive(0);
            }}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") {
                e.preventDefault();
                setActive((a) => Math.min(a + 1, options.length - 1));
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setActive((a) => Math.max(a - 1, 0));
              } else if (e.key === "Enter") {
                e.preventDefault();
                run(options[activeIndex]);
              } else if (e.key === "Escape") {
                e.preventDefault();
                close();
              }
            }}
          />
        </div>
        <div className={styles.results} id="palette-list" role="listbox" ref={listRef}>
          {options.length === 0 && <div className={styles.empty}>{paletteCopy.noResults}</div>}
          {options.map((opt, index) => {
            const section = opt.type === "item" ? paletteCopy.cards : paletteCopy[opt.cmd.section];
            const header = section !== lastSection ? section : null;
            lastSection = section;
            const key = opt.type === "item" ? `item:${opt.item.id}` : opt.cmd.id;
            const label = opt.type === "item" ? opt.item.title : opt.cmd.label;
            const icon =
              opt.type === "item"
                ? (() => {
                    const { Icon, color } = KIND_STYLE[opt.item.kind];
                    return (
                      <span style={{ color, display: "inline-flex" }}>
                        <Icon size={15} strokeWidth={1.75} />
                      </span>
                    );
                  })()
                : opt.icon;
            return (
              <div key={key}>
                {header && <div className={styles.section}>{header}</div>}
                <button
                  type="button"
                  role="option"
                  data-index={index}
                  aria-selected={index === activeIndex}
                  className={styles.option}
                  onMouseMove={() => index !== activeIndex && setActive(index)}
                  onClick={() => run(opt)}
                >
                  <span className={styles.optionIcon} aria-hidden="true">
                    {icon}
                  </span>
                  <span className={styles.optionLabel}>{label}</span>
                  {opt.type === "command" && opt.cmd.hint && (
                    <span className={styles.optionHint}>{opt.cmd.hint}</span>
                  )}
                  {opt.type === "item" && (
                    <span className={styles.optionHint}>{kindLabel[opt.item.kind]}</span>
                  )}
                </button>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
