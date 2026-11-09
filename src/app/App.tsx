import { useEffect } from "react";
import type { Board, Project } from "../api/types";
import { copy } from "../copy";
import { Header } from "../components/Header/Header";
import { Sidebar } from "../components/Sidebar/Sidebar";
import { useItemsStore } from "../stores/items";
import { useProjectsStore } from "../stores/projects";
import { useUiStore, type NavTarget } from "../stores/ui";
import styles from "./App.module.css";
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

export function App() {
  const target = useUiStore((s) => s.target);
  const projects = useProjectsStore((s) => s.projects);
  const boards = useProjectsStore((s) => s.boards);
  const loadProjects = useProjectsStore((s) => s.load);
  const order = useItemsStore((s) => s.order);
  const loadItems = useItemsStore((s) => s.load);
  const refreshCounts = useItemsStore((s) => s.refreshCounts);
  const subscribe = useItemsStore((s) => s.subscribe);

  useEffect(() => {
    void loadProjects();
    void refreshCounts();
    let unlisten: (() => void) | undefined;
    void subscribe().then((fn) => (unlisten = fn));
    return () => unlisten?.();
  }, [loadProjects, refreshCounts, subscribe]);

  useEffect(() => {
    if (target.kind === "settings") return;
    void loadItems(target.kind === "upcoming" ? { kind: "upcoming", days: 7 } : target);
  }, [target, loadItems]);

  const color =
    target.kind === "project" ? projects.find((p) => p.id === target.id)?.color : undefined;

  return (
    <div role="application" aria-label={copy.appName} className={styles.app}>
      <Sidebar />
      <main className={styles.main}>
        <Header title={titleFor(target, projects, boards)} count={order.length} color={color} />
        <Workspace target={target} />
      </main>
    </div>
  );
}
