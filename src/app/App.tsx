import { copy } from "../copy";
import { Header } from "../components/Header/Header";
import { Sidebar, type SidebarBoard, type SidebarProject } from "../components/Sidebar/Sidebar";
import { useUiStore, type NavTarget } from "../stores/ui";
import styles from "./App.module.css";
import { Workspace } from "./Workspace";

// Phase 0 ships the shell with no data; Task 8 wires these to the stores.
const EMPTY_COUNTS = { inbox: 0, today: 0, upcoming: 0, ideas: 0, notes: 0, byProject: {} };
const NO_PROJECTS: SidebarProject[] = [];
const NO_BOARDS: SidebarBoard[] = [];

function titleFor(target: NavTarget, projects: SidebarProject[], boards: SidebarBoard[]) {
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
  const color =
    target.kind === "project" ? NO_PROJECTS.find((p) => p.id === target.id)?.color : undefined;

  return (
    <div role="application" aria-label={copy.appName} className={styles.app}>
      <Sidebar counts={EMPTY_COUNTS} projects={NO_PROJECTS} boards={NO_BOARDS} />
      <main className={styles.main}>
        <Header title={titleFor(target, NO_PROJECTS, NO_BOARDS)} count={0} color={color} />
        <Workspace target={target} />
      </main>
    </div>
  );
}
