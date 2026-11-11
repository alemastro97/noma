import { ListFilter, Search, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { listTags } from "../../api/items";
import { ITEM_TYPES, STATUSES, type ItemType, type Status } from "../../api/types";
import { kindLabel, statusLabel, workspaceCopy } from "../../copy";
import { activeConditions } from "../../lib/filterSpec";
import { useFiltersStore } from "../../stores/filters";
import { useProjectsStore } from "../../stores/projects";
import { useUiStore } from "../../stores/ui";
import { useWorkspaceData } from "../../views/useWorkspace";
import { KIND_STYLE } from "../Card/kind";
import styles from "./FilterBar.module.css";

function toggle<T>(list: T[] | undefined, value: T): T[] {
  const cur = list ?? [];
  return cur.includes(value) ? cur.filter((v) => v !== value) : [...cur, value];
}

/** Header button that opens the filter bar; shows how many conditions are set. */
export function FilterToggle() {
  const open = useFiltersStore((s) => s.open);
  const n = useFiltersStore((s) => activeConditions(s.spec));
  const setOpen = useFiltersStore((s) => s.setOpen);
  return (
    <button
      type="button"
      className={styles.toggle}
      aria-pressed={open}
      aria-label={workspaceCopy.filterTitle}
      title={workspaceCopy.filterTitle}
      onClick={() => setOpen(!open)}
    >
      <ListFilter size={15} strokeWidth={1.75} aria-hidden="true" />
      {n > 0 && <span className={styles.badge}>{n}</span>}
    </button>
  );
}

function SaveFilter() {
  const [naming, setNaming] = useState(false);
  const [name, setName] = useState("");
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (naming) ref.current?.focus();
  }, [naming]);

  const submit = async () => {
    const value = name.trim();
    setNaming(false);
    setName("");
    if (!value) return;
    const saved = await useFiltersStore.getState().save(value);
    if (saved) useUiStore.getState().setTarget({ kind: "filter", id: saved.id });
  };

  if (!naming)
    return (
      <button type="button" className={styles.action} onClick={() => setNaming(true)}>
        {workspaceCopy.saveFilter}
      </button>
    );
  return (
    <input
      ref={ref}
      className={styles.nameInput}
      placeholder={workspaceCopy.filterName}
      aria-label={workspaceCopy.filterName}
      value={name}
      onChange={(e) => setName(e.target.value)}
      onBlur={() => void submit()}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          void submit();
        } else if (e.key === "Escape") {
          e.preventDefault();
          e.stopPropagation();
          setName("");
          setNaming(false);
        }
      }}
    />
  );
}

/** Conditions over the current view: text, type, status, project, tag, date range. */
export function FilterBar() {
  const open = useFiltersStore((s) => s.open);
  const spec = useFiltersStore((s) => s.spec);
  const setSpec = useFiltersStore((s) => s.setSpec);
  const projects = useProjectsStore((s) => s.projects);
  const { items } = useWorkspaceData();
  const [tags, setTags] = useState<string[]>([]);

  useEffect(() => {
    if (!open) return;
    listTags()
      .then((t) => setTags(t ?? []))
      .catch(() => setTags([]));
  }, [open]);

  if (!open) return null;
  const n = activeConditions(spec);

  return (
    <div className={styles.bar} role="search" aria-label={workspaceCopy.filters}>
      <label className={styles.search}>
        <Search size={14} strokeWidth={1.75} aria-hidden="true" />
        <input
          type="search"
          placeholder={workspaceCopy.filterText}
          aria-label={workspaceCopy.filterText}
          value={spec.text ?? ""}
          onChange={(e) => setSpec({ text: e.target.value })}
        />
      </label>

      <div className={styles.chips} role="group" aria-label={workspaceCopy.filterTypes}>
        {ITEM_TYPES.map((t: ItemType) => (
          <button
            key={t}
            type="button"
            className={styles.chip}
            aria-pressed={spec.types?.includes(t) ?? false}
            onClick={() => setSpec({ types: toggle(spec.types, t) })}
          >
            <span
              className={styles.dot}
              style={{ background: KIND_STYLE[t].color }}
              aria-hidden="true"
            />
            {kindLabel[t]}
          </button>
        ))}
      </div>

      <div className={styles.chips} role="group" aria-label={workspaceCopy.filterStatus}>
        {STATUSES.map((st: Status) => (
          <button
            key={st}
            type="button"
            className={styles.chip}
            aria-pressed={spec.statuses?.includes(st) ?? false}
            onClick={() => setSpec({ statuses: toggle(spec.statuses, st) })}
          >
            {statusLabel[st]}
          </button>
        ))}
      </div>

      <select
        className={styles.select}
        aria-label={workspaceCopy.filterProject}
        value={spec.projectIds?.[0] ?? ""}
        onChange={(e) => setSpec({ projectIds: e.target.value ? [e.target.value] : [] })}
      >
        <option value="">{workspaceCopy.anyProject}</option>
        <option value="none">{workspaceCopy.noProject}</option>
        {projects.map((p) => (
          <option key={p.id} value={p.id}>
            {p.name}
          </option>
        ))}
      </select>

      <select
        className={styles.select}
        aria-label={workspaceCopy.filterTag}
        value={spec.tags?.[0] ?? ""}
        onChange={(e) => setSpec({ tags: e.target.value ? [e.target.value] : [] })}
      >
        <option value="">{workspaceCopy.anyTag}</option>
        {tags.map((t) => (
          <option key={t} value={t}>
            #{t}
          </option>
        ))}
      </select>

      <label className={styles.date}>
        <span>{workspaceCopy.filterFrom}</span>
        <input
          type="date"
          value={spec.from ?? ""}
          onChange={(e) => setSpec({ from: e.target.value || undefined })}
        />
      </label>
      <label className={styles.date}>
        <span>{workspaceCopy.filterTo}</span>
        <input
          type="date"
          value={spec.to ?? ""}
          onChange={(e) => setSpec({ to: e.target.value || undefined })}
        />
      </label>

      <div className={styles.end}>
        {n > 0 && <span className={styles.meta}>{workspaceCopy.matches(items.length)}</span>}
        {n > 0 && (
          <button
            type="button"
            className={styles.action}
            onClick={() => useFiltersStore.getState().reset()}
          >
            {workspaceCopy.clearFilter}
          </button>
        )}
        {n > 0 && <SaveFilter />}
        <button
          type="button"
          className={styles.close}
          aria-label={workspaceCopy.closeFilter}
          title={workspaceCopy.closeFilter}
          onClick={() => useFiltersStore.getState().setOpen(false)}
        >
          <X size={14} strokeWidth={1.75} aria-hidden="true" />
        </button>
      </div>
    </div>
  );
}
