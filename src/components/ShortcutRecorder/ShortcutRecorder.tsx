import { useCallback, useEffect, useState } from "react";
import { shortcutsReset, shortcutsResume, shortcutsSet, shortcutsSuspend } from "../../api/desktop";
import { settingsCopy as t } from "../../copy";
import {
  acceleratorFromEvent,
  acceleratorParts,
  DEFAULT_SHORTCUTS,
  heldModifiers,
  isComplete,
  type Action,
} from "../../lib/shortcuts";
import styles from "./ShortcutRecorder.module.css";

function Keys({ parts, live }: { parts: string[]; live?: boolean }) {
  return (
    <span className={styles.keys} data-live={live || undefined}>
      {parts.map((p, i) => (
        <kbd key={`${p}${i}`} className={styles.key}>
          {p}
        </kbd>
      ))}
    </span>
  );
}

/**
 * Game-style key binding: press "Change", hold your combination (shown live),
 * then Save. The global shortcuts are released while recording.
 */
export function ShortcutRecorder({
  action,
  value,
  onChange,
}: {
  action: Action;
  value: string;
  onChange: (value: string) => void;
}) {
  const [recording, setRecording] = useState(false);
  const [mods, setMods] = useState<string[]>([]);
  const [candidate, setCandidate] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const stop = useCallback(() => {
    setRecording(false);
    setMods([]);
    setCandidate(null);
    void shortcutsResume().catch(() => undefined);
  }, []);

  const start = () => {
    setError(null);
    setCandidate(null);
    setMods([]);
    setRecording(true);
    void shortcutsSuspend().catch(() => undefined);
  };

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopImmediatePropagation();
      if (e.type === "keyup") {
        if (!candidate) setMods(heldModifiers(e));
        return;
      }
      if (e.code === "Escape") {
        stop();
        return;
      }
      if (e.repeat) return;
      const acc = acceleratorFromEvent(e);
      setMods(heldModifiers(e));
      if (acc) {
        setCandidate(acc);
        setError(isComplete(acc) ? null : t.shortcutNeedsModifier);
      }
    };
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("keyup", onKey, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("keyup", onKey, true);
    };
  }, [recording, candidate, stop]);

  // Never leave the shortcuts released if Settings closes mid-recording.
  useEffect(
    () => () => {
      void shortcutsResume().catch(() => undefined);
    },
    [],
  );

  const save = async () => {
    if (!isComplete(candidate)) return;
    try {
      const map = await shortcutsSet(action, candidate);
      onChange(map[action]);
      stop();
    } catch (err) {
      setError(typeof err === "string" ? err : String(err));
    }
  };

  const reset = async () => {
    try {
      const map = await shortcutsReset(action);
      onChange(map[action]);
      setError(null);
    } catch (err) {
      setError(typeof err === "string" ? err : String(err));
    }
  };

  const liveParts = candidate
    ? acceleratorParts(candidate)
    : mods.length
      ? acceleratorParts(mods.join("+"))
      : [];

  return (
    <div className={styles.root}>
      <div className={styles.line}>
        {recording ? (
          <span className={styles.capture} aria-live="polite">
            {liveParts.length ? (
              <Keys parts={liveParts} live />
            ) : (
              <span className={styles.prompt}>{t.shortcutPress}</span>
            )}
          </span>
        ) : (
          <Keys parts={acceleratorParts(value)} />
        )}

        {recording ? (
          <span className={styles.actions}>
            <button
              type="button"
              className={styles.save}
              disabled={!isComplete(candidate)}
              onClick={() => void save()}
            >
              {t.shortcutSave}
            </button>
            <button type="button" className={styles.ghost} onClick={stop}>
              {t.cancel}
            </button>
          </span>
        ) : (
          <span className={styles.actions}>
            <button type="button" className={styles.ghost} onClick={start}>
              {t.shortcutChange}
            </button>
            {value !== DEFAULT_SHORTCUTS[action] && (
              <button type="button" className={styles.ghost} onClick={() => void reset()}>
                {t.shortcutReset}
              </button>
            )}
          </span>
        )}
      </div>
      {error && (
        <span className={styles.error} role="alert">
          {error}
        </span>
      )}
    </div>
  );
}
