/** Global-shortcut helpers. Accelerators look like `Alt+Space` / `Super+Shift+KeyN`. */

export type Action = "voice" | "capture" | "today" | "search" | "lastCapture";

export const DEFAULT_SHORTCUTS: Record<Action, string> = {
  voice: "Alt+Space",
  capture: "Super+Shift+KeyN",
  // Unset ("") by default: any obvious combination would clash with other apps.
  today: "",
  search: "",
  lastCapture: "",
};

const MODIFIER_CODES = new Set([
  "AltLeft",
  "AltRight",
  "MetaLeft",
  "MetaRight",
  "ControlLeft",
  "ControlRight",
  "ShiftLeft",
  "ShiftRight",
]);

const SYMBOL: Record<string, string> = {
  Control: "⌃",
  Alt: "⌥",
  Shift: "⇧",
  Super: "⌘",
};

const KEY_LABEL: Record<string, string> = {
  Space: "Space",
  Enter: "↵",
  Tab: "Tab",
  Backspace: "⌫",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Semicolon: ";",
  Quote: "'",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
};

export interface KeyLike {
  code: string;
  altKey: boolean;
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
}

/** Modifier names held in a keyboard event, in display order. */
export function heldModifiers(e: KeyLike): string[] {
  const mods: string[] = [];
  if (e.ctrlKey) mods.push("Control");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Super");
  return mods;
}

/** The accelerator for a key press, or null while only modifiers are down. */
export function acceleratorFromEvent(e: KeyLike): string | null {
  if (MODIFIER_CODES.has(e.code)) return null;
  return [...heldModifiers(e), e.code].join("+");
}

function keyLabel(code: string): string {
  if (KEY_LABEL[code]) return KEY_LABEL[code];
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  if (code.startsWith("Numpad")) return `Num ${code.slice(6)}`;
  return code; // F1…F20, Home, End, …
}

/** Symbols for each part: `["⌥", "Space"]`, `["⌘", "⇧", "N"]`. */
export function acceleratorParts(accelerator: string): string[] {
  return accelerator
    .split("+")
    .filter(Boolean)
    .map((p) => SYMBOL[p] ?? keyLabel(p));
}

/** Compact text for copy: `⌥ Space`, `⌘⇧N`. */
export function formatAccelerator(accelerator: string): string {
  const parts = acceleratorParts(accelerator);
  const key = parts[parts.length - 1] ?? "";
  const mods = parts.slice(0, -1).join("");
  return key.length > 1 ? `${mods} ${key}` : `${mods}${key}`;
}

/** True when the accelerator has at least one modifier and a non-modifier key. */
export function isComplete(accelerator: string | null): accelerator is string {
  if (!accelerator) return false;
  const parts = accelerator.split("+");
  return parts.length >= 2 && !(parts[parts.length - 1]! in SYMBOL);
}
