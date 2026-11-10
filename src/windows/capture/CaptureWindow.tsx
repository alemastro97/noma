import { Check, Plus } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { captureText } from "../../api/ai";
import { panelHide } from "../../api/desktop";
import { desktopCopy } from "../../copy";
import { useShortcutLabels } from "../../hooks/useShortcuts";
import glass from "../shared/glass.module.css";
import styles from "./capture.module.css";

/** Quick capture: type a thought anywhere, NOMA files it (Auto mode). */
export function CaptureWindow() {
  const keys = useShortcutLabels();
  const [value, setValue] = useState("");
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const timer = useRef<number | undefined>(undefined);

  const reset = useCallback(() => {
    window.clearTimeout(timer.current);
    setSaved(false);
    setError(null);
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    window.addEventListener("focus", reset);
    return () => window.removeEventListener("focus", reset);
  }, [reset]);

  const close = useCallback(() => {
    window.clearTimeout(timer.current);
    void panelHide("capture");
  }, []);

  const submit = useCallback(async () => {
    const text = value.trim();
    if (!text) return;
    try {
      await captureText(text, {});
      setValue("");
      setSaved(true);
      timer.current = window.setTimeout(() => {
        setSaved(false);
        close();
      }, 700);
    } catch (err) {
      setError(String(err));
    }
  }, [value, close]);

  return (
    <section
      className={`${glass.glass} ${styles.panel}`}
      aria-label="Quick capture"
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.preventDefault();
          close();
        }
      }}
    >
      <label className={styles.field}>
        {saved ? (
          <Check size={18} className={styles.ok} aria-hidden="true" />
        ) : (
          <Plus size={18} className={styles.plus} aria-hidden="true" />
        )}
        <input
          ref={inputRef}
          type="text"
          aria-label="Quick capture"
          placeholder={saved ? desktopCopy.captureSaved : desktopCopy.capturePlaceholder}
          value={value}
          autoFocus
          onChange={(e) => {
            setValue(e.target.value);
            setError(null);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.nativeEvent.isComposing) {
              e.preventDefault();
              void submit();
            }
          }}
        />
      </label>
      <div className={styles.row}>
        <span className={`${glass.chip} ${styles.small} ${styles.auto}`}>{desktopCopy.auto}</span>
        <span className={`${glass.chip} ${styles.small} ${styles.muted}`}>{desktopCopy.inbox}</span>
        {error && <span className={styles.error}>{error}</span>}
        <span className={`${glass.kbd} ${styles.right}`}>{keys.capture}</span>
      </div>
    </section>
  );
}
