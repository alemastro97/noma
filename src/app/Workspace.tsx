import { InboxResort } from "../views/organize/InboxResort";
import { ProjectBrief } from "../views/organize/ProjectBrief";
import { ReviewView } from "../views/organize/ReviewView";
import { cardsCopy, copy, organizeCopy, workspaceCopy, uiCopy } from "../copy";
import { FilterBar } from "../components/FilterBar/FilterBar";
import { GraphView } from "../views/graph/GraphView";
import { EmptyState } from "../components/EmptyState/EmptyState";
import { useProjectsStore } from "../stores/projects";
import { targetKey, useUiStore, type NavTarget } from "../stores/ui";
import { CanvasView } from "../views/canvas/CanvasView";
import { KanbanView } from "../views/kanban/KanbanView";
import { ListView } from "../views/list/ListView";
import { SettingsView } from "../views/settings/SettingsView";
import { BinView } from "../views/archive/BinView";
import { CalendarView } from "../views/calendar/CalendarView";
import { HabitsSection } from "../views/habits/HabitsSection";
import { TimelineView } from "../views/timeline/TimelineView";
import { boardIdFor } from "./boardFor";
import styles from "./Workspace.module.css";

export function Workspace({ target }: { target: NavTarget }) {
  if (target.kind === "archive" || target.kind === "trash") {
    return (
      <section className={styles.views} aria-label={cardsCopy.nav[target.kind]}>
        <BinView key={target.kind} mode={target.kind} />
      </section>
    );
  }
  return <PlaceWorkspace target={target} />;
}

function PlaceWorkspace({ target }: { target: Exclude<NavTarget, { kind: "archive" | "trash" }> }) {
  const viewMode = useUiStore((s) => s.viewMode);
  const projects = useProjectsStore((s) => s.projects);
  if (target.kind === "review") {
    return (
      <section className={styles.views} aria-label={organizeCopy.review.nav}>
        <ReviewView />
      </section>
    );
  }
  const kind = target.kind;
  const text = kind === "filter" ? workspaceCopy.filterEmpty : copy.empty[kind];
  const empty = (
    <EmptyState
      title={text.title}
      body={text.body}
      variant={kind === "project" || kind === "board" ? "dashed" : "card"}
    />
  );

  if (target.kind === "settings") {
    return (
      <section className={styles.views} aria-label={uiCopy.regions.settings}>
        <SettingsView />
      </section>
    );
  }

  const projectBoards = Object.fromEntries(projects.map((p) => [p.id, p.boardId]));
  const key = targetKey(target);

  return (
    <section className={styles.views} aria-label={uiCopy.regions.workspace}>
      {target.kind === "inbox" && <InboxResort />}
      {target.kind === "project" && <ProjectBrief key={target.id} projectId={target.id} />}
      <FilterBar />
      {target.kind === "today" && (viewMode === "list" || viewMode === "kanban") && (
        <HabitsSection />
      )}
      {viewMode === "graph" ? (
        <GraphView key={key} empty={empty} />
      ) : viewMode === "calendar" ? (
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
