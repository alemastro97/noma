import { useEffect, useRef, useState } from "react";
import { cardsCopy } from "../../copy";
import { Markdown } from "./Markdown";
import styles from "./Cards.module.css";

/**
 * Card notes: rendered as Markdown, edited as plain text. Clicking the preview
 * (or an empty note) switches to the editor; leaving it saves.
 */
export function NotesField({
  value,
  className,
  label,
  placeholder,
  onSave,
}: {
  value: string;
  /** The detail panel's textarea style. */
  className?: string;
  label: string;
  placeholder: string;
  onSave: (text: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);
  const [synced, setSynced] = useState(value);
  const ref = useRef<HTMLTextAreaElement>(null);
  if (value !== synced) {
    setSynced(value);
    if (!editing) setDraft(value);
  }

  useEffect(() => {
    if (editing) ref.current?.focus();
  }, [editing]);

  if (!editing && value.trim()) {
    return (
      <div
        className={styles.notesPreview}
        role="button"
        tabIndex={0}
        aria-label={cardsCopy.editNotes}
        title={cardsCopy.editNotes}
        onClick={(e) => {
          // Let link clicks through without entering edit mode.
          if ((e.target as HTMLElement).closest("a")) return;
          setEditing(true);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            setEditing(true);
          }
        }}
      >
        <Markdown text={value} />
      </div>
    );
  }

  return (
    <>
      <textarea
        ref={ref}
        className={className}
        aria-label={label}
        placeholder={placeholder}
        value={draft}
        onFocus={() => setEditing(true)}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={() => {
          setEditing(false);
          if (draft !== value) onSave(draft);
        }}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.stopPropagation();
            (e.target as HTMLTextAreaElement).blur();
          }
        }}
      />
      {editing && <span className={styles.notesHint}>{cardsCopy.markdownHint}</span>}
    </>
  );
}
