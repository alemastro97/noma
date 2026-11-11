import { useEffect, useState } from "react";
import { DEFAULT_PRESENCE, onPresence, presenceGet, type PresenceState } from "../api/presence";

/** Live desktop presence (attention glow, quiet hours, orb look). */
export function usePresence(): PresenceState {
  const [state, setState] = useState<PresenceState>(DEFAULT_PRESENCE);
  useEffect(() => {
    let alive = true;
    presenceGet()
      .then((s) => alive && s && setState(s))
      .catch(() => undefined);
    const un = onPresence(setState);
    return () => {
      alive = false;
      void un.then((f) => f()).catch(() => undefined);
    };
  }, []);
  return state;
}

/** Orb diameter in pixels for each size (the window stays 176 px). */
export const ORB_PX = { s: 40, m: 52, l: 68 } as const;
