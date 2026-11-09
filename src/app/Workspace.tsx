import { copy, type EmptyKind } from "../copy";
import { EmptyState } from "../components/EmptyState/EmptyState";
import type { NavTarget } from "../stores/ui";
import styles from "./Workspace.module.css";

interface WorkspaceProps {
  target: NavTarget;
}

/** Main area. Phase 0 renders the empty state for the active target; views arrive in Phase 1. */
export function Workspace({ target }: WorkspaceProps) {
  const kind: EmptyKind = target.kind;
  const text = copy.empty[kind];
  return (
    <section className={styles.workspace} aria-label="Workspace">
      <EmptyState
        title={text.title}
        body={text.body}
        variant={kind === "project" || kind === "board" ? "dashed" : "card"}
      />
    </section>
  );
}
