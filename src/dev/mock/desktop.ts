/** Mock backend cases for the desktop presence extras (orb look, quiet hours, panel). */
import { DEFAULT_PRESENCE, type PresenceState } from "../../api/presence";

/** Returns undefined when the command is not one of ours. */
export function handleDesktop(cmd: string, a: Record<string, unknown>): unknown {
  switch (cmd) {
    case "presence_get": {
      // A demo glow in the browser: `?attention` in the URL.
      const state: PresenceState = {
        ...DEFAULT_PRESENCE,
        attention: location.search.includes("attention"),
      };
      return state;
    }
    case "presence_open_item":
      console.info(`[mock backend] open item ${String(a.itemId)}`);
      return null;
    default:
      return undefined;
  }
}
