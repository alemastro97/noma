import { copy, organizeCopy, type EmptyKind } from "../copy";
import { InboxResort } from "../views/organize/InboxResort";
import { ProjectBrief } from "../views/organize/ProjectBrief";
import { ReviewView } from "../views/organize/ReviewView";
import { EmptyState } from "../components/EmptyState/EmptyState";
import { useProjectsStore } from "../stores/projects";
import { targetKey, useUiStore, type NavTarget } from "../stores/ui";
import { CanvasView } from "../views/canvas/CanvasView";
import { KanbanView } from "../views/kanban/KanbanView";
import { ListView } from "../views/list/ListView";
import { SettingsView } from "../views/settings/SettingsView";
import { boardIdFor } from "./boardFor";
import styles from "./Workspace.module.css";

export function Workspace({ target }: { target: NavTarget }) {
  const viewMode = useUiStore((s) => s.viewMode);
  const projects = useProjectsStore((s) => s.projects);
  if (target.kind === "review") {
    return (
      <section className={styles.views} aria-label={organizeCopy.review.nav}>
        <ReviewView />
      </section>
    );
  }
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
      {target.kind === "inbox" && <InboxResort />}
      {target.kind === "project" && <ProjectBrief key={target.id} projectId={target.id} />}
      {viewMode === "kanban" ? (
        <KanbanView key={key} empty={empty} />
      ) : viewMode === "list" ? (
        <ListView key={key} mode={target.kind === "today" ? "today" : "timeline"} empty={empty} />
      ) : (
        <CanvasView boardId={boardIdFor(target, projectBoards)} viewKey={key} empty={empty} />
      )}
    </section>
  );
}
