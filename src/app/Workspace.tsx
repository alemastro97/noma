import { copy, type EmptyKind } from "../copy";
import { EmptyState } from "../components/EmptyState/EmptyState";
import { useProjectsStore } from "../stores/projects";
import { targetKey, useUiStore, type NavTarget } from "../stores/ui";
import { CanvasView } from "../views/canvas/CanvasView";
import { KanbanView } from "../views/kanban/KanbanView";
import { ListView } from "../views/list/ListView";
import { SettingsView } from "../views/settings/SettingsView";
import { CalendarView } from "../views/calendar/CalendarView";
import { HabitsSection } from "../views/habits/HabitsSection";
import { TimelineView } from "../views/timeline/TimelineView";
import { boardIdFor } from "./boardFor";
import styles from "./Workspace.module.css";

export function Workspace({ target }: { target: NavTarget }) {
  const viewMode = useUiStore((s) => s.viewMode);
  const projects = useProjectsStore((s) => s.projects);
  const kind: EmptyKind = target.kind;
  const text = copy.empty[kind];
  const empty = (
    <EmptyState
      title={text.title}
      body={text.body}
      variant={kind === "project" || kind === "board" ? "dashed" : "card"}
    />
  );

  if (target.kind === "settings") {
    return (
      <section className={styles.views} aria-label="Settings">
        <SettingsView />
      </section>
    );
  }

  const projectBoards = Object.fromEntries(projects.map((p) => [p.id, p.boardId]));
  const key = targetKey(target);

  return (
    <section className={styles.views} aria-label="Workspace">
      {target.kind === "today" && (viewMode === "list" || viewMode === "kanban") && (
        <HabitsSection />
      )}
      {viewMode === "calendar" ? (
        <CalendarView key={key} />
      ) : viewMode === "timeline" ? (
        <TimelineView key={key} />
      ) : viewMode === "kanban" ? (
        <KanbanView key={key} empty={empty} />
      ) : viewMode === "list" ? (
        <ListView key={key} mode={target.kind === "today" ? "today" : "timeline"} empty={empty} />
      ) : (
        <CanvasView boardId={boardIdFor(target, projectBoards)} viewKey={key} empty={empty} />
      )}
    </section>
  );
}
