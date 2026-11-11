import { useCallback, useEffect, useRef, useState } from "react";
import { organizeCopy } from "../../copy";
import { toastError } from "../../stores/toasts";

/**
 * Runs a model-backed action with a busy flag. Failures show one friendly
 * toast (the backend's message goes to the console, not to the user).
 */
export function useAiAction<A extends unknown[], R>(
  run: (...args: A) => Promise<R>,
  failure: string = organizeCopy.aiFailed,
): { busy: boolean; go: (...args: A) => Promise<R | null> } {
  const [busy, setBusy] = useState(false);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  const go = useCallback(
    async (...args: A) => {
      setBusy(true);
      try {
        return await run(...args);
      } catch (err) {
        console.warn("[organize]", err);
        toastError(failure);
        return null;
      } finally {
        if (alive.current) setBusy(false);
      }
    },
    [run, failure],
  );
  return { busy, go };
}
