import { AlertCircle, CalendarClock, FolderPlus, X } from "lucide-react";
import type { MouseEvent } from "react";
import type { CaptureHint, Item } from "../../api/types";
import { hintCopy } from "../../copy";
import { nextProjectColor } from "../../lib/colors";
import { useItemsStore } from "../../stores/items";
import { useProjectsStore } from "../../stores/projects";
import styles from "./Card.module.css";

function without(hint: CaptureHint, key: keyof CaptureHint): CaptureHint | null {
  const next = { ...hint };
  delete next[key];
  return next.datePhrase || next.lowConfidence || next.suggestedProject ? next : null;
}

/** Review chips left by AI capture: check the date, confirm a new project, check the card. */
export function HintChips({ item }: { item: Item }) {
  const hint = item.hint;
  if (!hint) return null;
  const update = useItemsStore.getState().update;
  const dismiss = (key: keyof CaptureHint) => (e: MouseEvent) => {
    e.stopPropagation();
    void update(item.id, { hint: without(hint, key) });
  };

  const createProject = async (e: MouseEvent) => {
    e.stopPropagation();
    const name = hint.suggestedProject;
    if (!name) return;
    const store = useProjectsStore.getState();
    const project = await store.createProject(name, nextProjectColor(store.projects.length));
    if (project)
      await update(item.id, { projectId: project.id, hint: without(hint, "suggestedProject") });
  };

  return (
    <div className={styles.hints} onDoubleClick={(e) => e.stopPropagation()}>
      {hint.datePhrase && (
        <span className={styles.hint} data-tone="date">
          <CalendarClock size={12} strokeWidth={2} aria-hidden="true" />
          {hintCopy.checkDate(hint.datePhrase)}
          <button type="button" aria-label={hintCopy.dismiss} onClick={dismiss("datePhrase")}>
            <X size={11} strokeWidth={2} aria-hidden="true" />
          </button>
        </span>
      )}
      {hint.suggestedProject && (
        <span className={styles.hint} data-tone="project">
          <button
            type="button"
            className={styles.hintAction}
            onClick={(e) => void createProject(e)}
          >
            <FolderPlus size={12} strokeWidth={2} aria-hidden="true" />
            {hintCopy.newProject(hint.suggestedProject)}
          </button>
          <button type="button" aria-label={hintCopy.dismiss} onClick={dismiss("suggestedProject")}>
            <X size={11} strokeWidth={2} aria-hidden="true" />
          </button>
        </span>
      )}
      {hint.lowConfidence && (
        <span className={styles.hint}>
          <AlertCircle size={12} strokeWidth={2} aria-hidden="true" />
          {hintCopy.checkThis}
          <button type="button" aria-label={hintCopy.dismiss} onClick={dismiss("lowConfidence")}>
            <X size={11} strokeWidth={2} aria-hidden="true" />
          </button>
        </span>
      )}
    </div>
  );
}
