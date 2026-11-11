import { useCallback, useEffect, useState } from "react";
import { focusCurrent, focusStart, focusStop, onFocusChanged, type FocusState } from "../api/time";
import { toDate } from "../lib/dates";

/** Default focus session length. */
export const FOCUS_MINUTES = 25;

export interface FocusClock {
  focus: FocusState | null;
  /** Milliseconds left, 0 when idle. */
  leftMs: number;
  /** 0 → 1 through the session. */
  progress: number;
  start: (itemId: string) => void;
  stop: () => void;
}

/** "24:59". */
export function clock(ms: number): string {
  const total = Math.max(0, Math.ceil(ms / 1000));
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}

/** The running focus session (shared by every window), ticking once a second. */
export function useFocus(): FocusClock {
  const [focus, setFocus] = useState<FocusState | null>(null);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    let alive = true;
    void focusCurrent()
      .then((f) => alive && setFocus(f ?? null))
      .catch(() => undefined);
    const un = onFocusChanged((f) => setFocus(f ?? null));
    return () => {
      alive = false;
      void un.then((f) => f()).catch(() => undefined);
    };
  }, []);

  useEffect(() => {
    if (!focus) return;
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, [focus]);

  const start = useCallback((itemId: string) => {
    void focusStart(itemId, FOCUS_MINUTES)
      .then((f) => {
        setNow(Date.now());
        setFocus(f);
      })
      .catch(() => undefined);
  }, []);
  const stop = useCallback(() => {
    setFocus(null);
    void focusStop().catch(() => undefined);
  }, []);

  const ends = toDate(focus?.endsAt)?.getTime() ?? 0;
  const started = toDate(focus?.startedAt)?.getTime() ?? 0;
  const leftMs = focus ? Math.max(0, ends - now) : 0;
  const span = Math.max(1, ends - started);
  const progress = focus ? Math.min(1, Math.max(0, 1 - leftMs / span)) : 0;
  return { focus, leftMs, progress, start, stop };
}
