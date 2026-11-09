import { useCallback, useEffect, useRef, useState } from "react";
import type { OrbState } from "./NomaOrb";

export const SUCCESS_FLASH_MS = 1400;
export const ERROR_FLASH_MS = 2500;

export interface OrbStateController {
  state: OrbState;
  set: (state: OrbState) => void;
  flashSuccess: () => void;
  flashError: (ms?: number) => void;
}

/** Owns an orb's state and the timed return to idle after success/error flashes. */
export function useOrbState(initial: OrbState = "idle"): OrbStateController {
  const [state, setState] = useState<OrbState>(initial);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clear = useCallback(() => {
    if (timer.current !== null) {
      clearTimeout(timer.current);
      timer.current = null;
    }
  }, []);

  const set = useCallback(
    (next: OrbState) => {
      clear();
      setState(next);
    },
    [clear],
  );

  const flash = useCallback(
    (next: OrbState, ms: number) => {
      clear();
      setState(next);
      timer.current = setTimeout(() => {
        timer.current = null;
        setState("idle");
      }, ms);
    },
    [clear],
  );

  const flashSuccess = useCallback(() => flash("success", SUCCESS_FLASH_MS), [flash]);
  const flashError = useCallback((ms: number = ERROR_FLASH_MS) => flash("error", ms), [flash]);

  useEffect(() => clear, [clear]);

  return { state, set, flashSuccess, flashError };
}
