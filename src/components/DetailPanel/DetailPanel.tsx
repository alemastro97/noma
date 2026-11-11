import { Copy, Trash2, X } from "lucide-react";
import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { getItem } from "../../api/items";
import { ITEM_TYPES, PRIORITIES, STATUSES, type Item, type ItemPatch } from "../../api/types";
import { detailCopy, kindLabel, priorityLabel, statusLabel } from "../../copy";
import { anchorDate, editedAgo, fromLocalInput, toDate, toLocalInput } from "../../lib/dates";
import {
  RECURRENCE_PRESETS,
  presetFor,
  ruleFor,
  type RecurrencePreset,
} from "../../lib/recurrence";
import { useItemsStore } from "../../stores/items";
import { useProjectsStore } from "../../stores/projects";
import { useUiStore } from "../../stores/ui";
import { OrganizeActions } from "../../views/organize/OrganizeActions";
import { KIND_STYLE } from "../Card/kind";
import styles from "./DetailPanel.module.css";

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className={styles.field}>
      <span className={styles.label}>{label}</span>
      {children}
    </label>
  );
}

/** Text input that saves on blur / Enter instead of on every keystroke. */
function useDraft(value: string, save: (v: string) => void) {
  const [draft, setDraft] = useState(value);
  const [synced, setSynced] = useState(value);
  // Adopt server-side changes (adjusting state during render, not in an effect).
  if (value !== synced) {
    setSynced(value);
    setDraft(value);
  }
  const commit = () => {
    if (draft !== value) save(draft);
  };
  return { draft, setDraft, commit };
}

function TagsField({ tags, onChange }: { tags: string[]; onChange: (tags: string[]) => void }) {
  const [text, setText] = useState("");
  const add = () => {
    const t = text.trim().replace(/^#/, "");
    if (t && !tags.some((x) => x.toLowerCase() === t.toLowerCase())) onChange([...tags, t]);
    setText("");
  };
  return (
    <div className={styles.tags}>
      {tags.map((t) => (
        <span key={t} className={styles.tagChip}>
          #{t}
          <button
            type="button"
            aria-label={`Remove ${t}`}
            onClick={() => onChange(tags.filter((x) => x !== t))}
          >
            <X size={11} strokeWidth={2} aria-hidden="true" />
          </button>
        </span>
      ))}
      <input
        className={styles.tagInput}
        value={text}
        placeholder={detailCopy.tagsPlaceholder}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === ",") {
            e.preventDefault();
            add();
          } else if (e.key === "Backspace" && !text && tags.length) {
            onChange(tags.slice(0, -1));
          }
        }}
        onBlur={add}
      />
    </div>
  );
}

function DateField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string | null;
  onChange: (iso: string | null) => void;
}) {
  return (
    <Field label={label}>
      <div className={styles.dateRow}>
        <input
          type="datetime-local"
          className={styles.input}
          value={toLocalInput(value)}
          onChange={(e) => onChange(fromLocalInput(e.target.value))}
        />
        {value && (
          <button
            type="button"
            className={styles.clear}
            aria-label={`${detailCopy.clear} ${label}`}
            onClick={() => onChange(null)}
          >
            <X size={13} strokeWidth={1.75} aria-hidden="true" />
          </button>
        )}
      </div>
    </Field>
  );
}

function Editor({ item }: { item: Item }) {
  const projects = useProjectsStore((s) => s.projects);
  const update = (patch: ItemPatch) => void useItemsStore.getState().update(item.id, patch);
  const title = useDraft(item.title, (t) => t.trim() && update({ title: t.trim() }));
  const body = useDraft(item.body ?? "", (b) => update({ body: b.trim() ? b : null }));
  const titleRef = useRef<HTMLTextAreaElement>(null);
  const anchor = anchorDate(item);
  const created = toDate(item.createdAt);
  const isEvent = item.kind === "event";

  useEffect(() => {
    const el = titleRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [title.draft]);

  const onRepeat = (preset: RecurrencePreset) => {
    if (!anchor && preset !== "none") return;
    update({ recurrence: ruleFor(preset, anchor ?? new Date()) });
  };

  const onTitleKey = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter") {
      e.preventDefault();
      (e.target as HTMLTextAreaElement).blur();
    }
  };

  return (
    <div className={styles.body}>
      <div className={styles.kinds} role="radiogroup" aria-label={detailCopy.type}>
        {ITEM_TYPES.map((k) => {
          const { Icon, color } = KIND_STYLE[k];
          const active = item.kind === k;
          return (
            <button
              key={k}
              type="button"
              role="radio"
              aria-checked={active}
              className={styles.kindBtn}
              style={active ? { color, borderColor: color } : undefined}
              onClick={() => !active && update({ kind: k })}
              title={kindLabel[k]}
            >
              <Icon size={14} strokeWidth={1.75} aria-hidden={true} />
              <span>{kindLabel[k]}</span>
            </button>
          );
        })}
      </div>

      <textarea
        ref={titleRef}
        className={styles.title}
        aria-label={detailCopy.title}
        rows={1}
        value={title.draft}
        onChange={(e) => title.setDraft(e.target.value)}
        onBlur={title.commit}
        onKeyDown={onTitleKey}
        data-idea={item.kind === "idea" || undefined}
      />

      <textarea
        className={styles.notes}
        aria-label={detailCopy.notes}
        placeholder={detailCopy.notesPlaceholder}
        value={body.draft}
        onChange={(e) => body.setDraft(e.target.value)}
        onBlur={body.commit}
      />

      <OrganizeActions item={item} />

      <div className={styles.grid}>
        <Field label={detailCopy.status}>
          <select
            className={styles.input}
            value={item.status}
            onChange={(e) =>
              void useItemsStore.getState().setStatus(item.id, e.target.value as Item["status"])
            }
          >
            {STATUSES.map((s) => (
              <option key={s} value={s}>
                {statusLabel[s]}
              </option>
            ))}
          </select>
        </Field>
        <Field label={detailCopy.priority}>
          <select
            className={styles.input}
            value={item.priority}
            onChange={(e) => update({ priority: e.target.value as Item["priority"] })}
          >
            {PRIORITIES.map((p) => (
              <option key={p} value={p}>
                {priorityLabel[p]}
              </option>
            ))}
          </select>
        </Field>
      </div>

      <DateField
        label={isEvent ? detailCopy.starts : detailCopy.due}
        value={item.dueAt}
        onChange={(dueAt) => update({ dueAt })}
      />
      {isEvent && (
        <DateField
          label={detailCopy.ends}
          value={item.endsAt}
          onChange={(endsAt) => update({ endsAt })}
        />
      )}
      <DateField
        label={detailCopy.reminder}
        value={item.remindAt}
        onChange={(remindAt) => update({ remindAt })}
      />

      <Field label={detailCopy.repeat}>
        <select
          className={styles.input}
          value={presetFor(item.recurrence)}
          disabled={!anchor}
          title={anchor ? undefined : detailCopy.repeatNeedsDate}
          onChange={(e) => onRepeat(e.target.value as RecurrencePreset)}
        >
          {RECURRENCE_PRESETS.map((p) => (
            <option key={p.value} value={p.value}>
              {p.label}
            </option>
          ))}
        </select>
      </Field>

      <Field label={detailCopy.project}>
        <select
          className={styles.input}
          value={item.projectId ?? ""}
          onChange={(e) => update({ projectId: e.target.value || null })}
        >
          <option value="">{detailCopy.noProject}</option>
          {projects.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      </Field>

      <Field label={detailCopy.tags}>
        <TagsField tags={item.tags} onChange={(tags) => update({ tags })} />
      </Field>

      {created && (
        <p className={styles.footnote}>
          {detailCopy.created}{" "}
          {created.toLocaleDateString("en-GB", { day: "numeric", month: "short" })} ·{" "}
          {editedAgo(toDate(item.updatedAt) ?? created, new Date())}
        </p>
      )}
    </div>
  );
}

/** Slide-over editor for the card in `ui.detailId`. */
export function DetailPanel() {
  const detailId = useUiStore((s) => s.detailId);
  const close = useUiStore((s) => s.closeDetail);
  const fromStore = useItemsStore((s) => (detailId ? s.items[detailId] : undefined));
  const [fetched, setFetched] = useState<Item | null>(null);

  // Cards opened from search may not be in the current list: fetch them.
  useEffect(() => {
    if (!detailId || fromStore) return;
    let alive = true;
    void getItem(detailId).then((i) => {
      if (!alive) return;
      if (i) {
        useItemsStore.setState((s) => ({ items: { ...s.items, [i.id]: i } }));
        setFetched(i);
      } else close();
    });
    return () => {
      alive = false;
    };
  }, [detailId, fromStore, close]);

  const item = fromStore ?? (fetched?.id === detailId ? fetched : null);
  if (!detailId || !item) return null;

  return (
    <aside className={styles.panel} aria-label="Card details">
      <header className={styles.header}>
        <span className={styles.headerKind} style={{ color: KIND_STYLE[item.kind].color }}>
          {kindLabel[item.kind]}
        </span>
        <div className={styles.headerActions}>
          <button
            type="button"
            className={styles.iconBtn}
            aria-label="Duplicate"
            onClick={() => void useItemsStore.getState().duplicate(item.id)}
          >
            <Copy size={15} strokeWidth={1.75} aria-hidden="true" />
          </button>
          <button
            type="button"
            className={styles.iconBtn}
            aria-label="Delete"
            onClick={() => {
              useItemsStore.getState().remove(item.id);
              close();
            }}
          >
            <Trash2 size={15} strokeWidth={1.75} aria-hidden="true" />
          </button>
          <button
            type="button"
            className={styles.iconBtn}
            aria-label={detailCopy.close}
            onClick={close}
          >
            <X size={16} strokeWidth={1.75} aria-hidden="true" />
          </button>
        </div>
      </header>
      <Editor key={item.id} item={item} />
    </aside>
  );
}
