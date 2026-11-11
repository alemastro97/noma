import { useState } from "react";
import { ShortcutRecorder } from "../../components/ShortcutRecorder/ShortcutRecorder";
import { desktopPresenceCopy as t } from "../../copy";
import { useSetting } from "../../hooks/useSetting";
import { useShortcuts } from "../../hooks/useShortcuts";
import type { Action } from "../../lib/shortcuts";
import own from "./DesktopPresenceSettings.module.css";
import styles from "./Settings.module.css";

function Choice<T extends string>({
  label,
  hint,
  value,
  options,
  onChange,
}: {
  label: string;
  hint: string;
  value: string | null;
  options: Record<T, string>;
  onChange: (v: T) => void;
}) {
  return (
    <div className={styles.row}>
      <span className={styles.toggleText}>
        <span className={styles.toggleLabel}>{label}</span>
        <span className={styles.meta}>{hint}</span>
      </span>
      <div className={styles.segmented} role="radiogroup" aria-label={label}>
        {(Object.entries(options) as [T, string][]).map(([v, text]) => (
          <button
            key={v}
            type="button"
            role="radio"
            aria-checked={v === value}
            className={styles.segment}
            onClick={() => onChange(v)}
          >
            {text}
          </button>
        ))}
      </div>
    </div>
  );
}

function Switch({
  label,
  hint,
  value,
  onChange,
}: {
  label: string;
  hint: string;
  value: string | null;
  onChange: (on: boolean) => void;
}) {
  return (
    <label className={styles.toggleRow}>
      <span className={styles.toggleText}>
        <span className={styles.toggleLabel}>{label}</span>
        <span className={styles.meta}>{hint}</span>
      </span>
      <input
        type="checkbox"
        role="switch"
        className={styles.switch}
        checked={value === "1"}
        disabled={value === null}
        onChange={(e) => onChange(e.target.checked)}
      />
    </label>
  );
}

function Binding({ action, label }: { action: Action; label: string }) {
  const shortcuts = useShortcuts();
  const [override, setOverride] = useState<string | null>(null);
  return (
    <div className={styles.shortcut}>
      <span>{label}</span>
      <ShortcutRecorder
        action={action}
        value={override ?? shortcuts[action]}
        onChange={setOverride}
      />
    </div>
  );
}

/** Settings → On your desktop: menu-bar count, orb look, quiet hours, extra shortcuts. */
export function DesktopPresenceSettings() {
  const [count, setCount] = useSetting("ui.menubarCount", "1");
  const [size, setSize] = useSetting("orb.size", "m");
  const [style, setStyle] = useSetting("orb.style", "lively");
  const [quiet, setQuiet] = useSetting("reminders.quietEnabled", "0");
  const [from, setFrom] = useSetting("reminders.quietFrom", "22:00");
  const [to, setTo] = useSetting("reminders.quietTo", "08:00");

  return (
    <>
      <Switch
        label={t.menubarCount}
        hint={t.menubarCountHint}
        value={count}
        onChange={(on) => setCount(on ? "1" : "0")}
      />
      <Choice
        label={t.orbSize}
        hint={t.orbSizeHint}
        value={size}
        options={t.orbSizes}
        onChange={setSize}
      />
      <Choice
        label={t.orbStyle}
        hint={t.orbStyleHint}
        value={style}
        options={t.orbStyles}
        onChange={setStyle}
      />
      <Switch
        label={t.quiet}
        hint={t.quietHint}
        value={quiet}
        onChange={(on) => setQuiet(on ? "1" : "0")}
      />
      {quiet === "1" && (
        <div className={own.times}>
          <label className={own.time}>
            <span className={styles.meta}>{t.quietFrom}</span>
            <input
              type="time"
              className={styles.select}
              value={from ?? "22:00"}
              onChange={(e) => e.target.value && setFrom(e.target.value)}
            />
          </label>
          <label className={own.time}>
            <span className={styles.meta}>{t.quietTo}</span>
            <input
              type="time"
              className={styles.select}
              value={to ?? "08:00"}
              onChange={(e) => e.target.value && setTo(e.target.value)}
            />
          </label>
        </div>
      )}
      <div className={styles.shortcuts}>
        <span className={styles.label}>{t.moreShortcuts}</span>
        <p className={styles.meta}>{t.moreShortcutsHint}</p>
        <Binding action="today" label={t.shortcutToday} />
        <Binding action="search" label={t.shortcutSearch} />
        <Binding action="lastCapture" label={t.shortcutLastCapture} />
      </div>
    </>
  );
}
