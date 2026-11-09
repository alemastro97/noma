import { useCallback, useEffect, useState } from "react";
import { settingGet, settingSet } from "../api/desktop";

/**
 * A persisted UI setting (string-valued, backend `settings` table).
 * `value` is null until loaded; `fallback` is used when the key is unset.
 */
export function useSetting(key: string, fallback: string) {
  const [value, setValue] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    settingGet(key)
      .then((v) => alive && setValue(v ?? fallback))
      .catch(() => alive && setValue(fallback));
    return () => {
      alive = false;
    };
  }, [key, fallback]);

  const set = useCallback(
    (next: string) => {
      setValue(next);
      void settingSet(key, next).catch(() => undefined);
    },
    [key],
  );

  return [value, set] as const;
}
