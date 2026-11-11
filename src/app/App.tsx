import { useEffect, useMemo, useState } from "react";
import { onOpenItem, onOpenToday, settingGet } from "../api/desktop";
import { getItem } from "../api/items";
import { Onboarding } from "../components/Onboarding/Onboarding";
import type { Board, ItemFilter, Project } from "../api/types";
import { copy } from "../copy";
import { CommandPalette } from "../components/CommandPalette/CommandPalette";
import { DetailPanel } from "../components/DetailPanel/DetailPanel";
import { Header } from "../components/Header/Header";
import { QuickAdd } from "../components/QuickAdd/QuickAdd";
import { Sidebar } from "../components/Sidebar/Sidebar";
import { Toasts } from "../components/Toasts/Toasts";
import { useAiStore } from "../stores/ai";
import { useBoardStore } from "../stores/board";
import { flushPendingDeletes, useItemsStore } from "../stores/items";
import { useProjectsStore } from "../stores/projects";
import { useUiStore, type NavTarget } from "../stores/ui";
import styles from "./App.module.css";
import { useGlobalHotkeys } from "./useGlobalHotkeys";
import { boardIdFor } from "./boardFor";
import { Workspace } from "./Workspace";

function titleFor(target: NavTarget, projects: Project[], boards: Board[]) {
  switch (target.kind) {
    case "project":
      return projects.find((p) => p.id === target.id)?.name ?? copy.nav.projects;
    case "board":
      return boards.find((b) => b.id === target.id)?.name ?? copy.nav.boards;
    default:
      return copy.nav[target.kind];
  }
}

function filterFor(target: NavTarget): ItemFilter | null {
  switch (target.kind) {
    case "settings":
      return null;
    case "upcoming":
      return { kind: "upcoming", days: 7 };
    default:
      return target;
  }
}

export function App() {
  const target = useUiStore((s) => s.target);
  const viewMode = useUiStore((s) => s.viewMode);
  const projects = useProjectsStore((s) => s.projects);
  const boards = useProjectsStore((s) => s.boards);
  const order = useItemsStore((s) => s.order);
  const itemsById = useItemsStore((s) => s.items);

  useGlobalHotkeys();

  const [onboarding, setOnboarding] = useState(false);
  useEffect(() => {
    settingGet("onboarding.done")
      .then((v) => setOnboarding(typeof v !== "undefined" && v !== "1"))
      .catch(() => undefined);
  }, []);

  // "Open" on a filed toast: go to the card's place and show it.
  useEffect(() => {
    const un = onOpenItem((id) => {
      void getItem(id)
        .then((item) => {
          if (!item) return;
          const ui = useUiStore.getState();
          ui.setTarget(
            item.projectId ? { kind: "project", id: item.projectId } : { kind: "inbox" },
          );
          ui.openDetail(id);
        })
        .catch(() => undefined);
    });
    const unToday = onOpenToday(() => useUiStore.getState().setTarget({ kind: "today" }));
    return () => {
      void un.then((f) => f()).catch(() => undefined);
      void unToday.then((f) => f()).catch(() => undefined);
    };
  }, []);

  // Initial data and backend change events. Safe under StrictMode's double effect.
  useEffect(() => {
    void useProjectsStore.getState().load();
    void useItemsStore.getState().refreshCounts();
    void useAiStore.getState().load();
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    let unlistenAi: (() => void) | undefined;
    void useAiStore
      .getState()
      .subscribe()
      .then((fn) => {
        if (cancelled) fn();
        else unlistenAi = fn;
      })
      .catch(() => undefined);
    void useItemsStore
      .getState()
      .subscribe()
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      });
    window.addEventListener("beforeunload", flushPendingDeletes);
    return () => {
      cancelled = true;
      unlisten?.();
      unlistenAi?.();
      window.removeEventListener("beforeunload", flushPendingDeletes);
    };
  }, []);

  const filter = useMemo(() => filterFor(target), [target]);
  const includeDone = viewMode !== "canvas";

  useEffect(() => {
    if (filter) void useItemsStore.getState().load(filter, { includeDone });
  }, [filter, includeDone]);

  const projectBoards = useMemo(
    () => Object.fromEntries(projects.map((p) => [p.id, p.boardId])),
    [projects],
  );
  const boardId = boardIdFor(target, projectBoards);
  useEffect(() => {
    void useBoardStore.getState().load(viewMode === "canvas" ? boardId : null);
  }, [boardId, viewMode]);

  const color =
    target.kind === "project" ? projects.find((p) => p.id === target.id)?.color : undefined;
  const openCount = order.filter((id) => itemsById[id] && itemsById[id].status !== "done").length;

  return (
    <div role="application" aria-label={copy.appName} className={styles.app}>
      <Sidebar />
      <main className={styles.main}>
        <Header
          title={titleFor(target, projects, boards)}
          count={openCount}
          color={color}
          plain={target.kind === "settings"}
          onCapture={() => useUiStore.getState().openQuickAdd()}
          onSearch={() => useUiStore.getState().setPaletteOpen(true)}
        />
        <div className={styles.content}>
          <Workspace target={target} />
          <DetailPanel />
        </div>
      </main>
      <QuickAdd />
      <CommandPalette />
      <Toasts />
      {onboarding && <Onboarding onDone={() => setOnboarding(false)} />}
    </div>
  );
}
