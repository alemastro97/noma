import { useEffect, useRef } from "react";

const IS_MAC = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform ?? "");

interface Combo {
  key: string;
  mod: boolean;
  shift: boolean;
  alt: boolean;
}

export function parseCombo(combo: string): Combo {
  const parts = combo.toLowerCase().split("+");
  const key = parts.pop() ?? "";
  return {
    key: key === "space" ? " " : key,
    mod: parts.includes("mod"),
    shift: parts.includes("shift"),
    alt: parts.includes("alt") || parts.includes("option"),
  };
}

export function matchesCombo(e: KeyboardEvent, combo: Combo): boolean {
  const modPressed = IS_MAC ? e.metaKey : e.ctrlKey;
  return (
    e.key.toLowerCase() === combo.key &&
    modPressed === combo.mod &&
    e.shiftKey === combo.shift &&
    e.altKey === combo.alt
  );
}

/**
 * Runs `handler` when `combo` ("mod+k", "mod+\\", "mod+shift+n") is pressed anywhere in the window.
 * `mod` is ⌘ on macOS and Ctrl elsewhere.
 */
export function useHotkey(combo: string, handler: () => void): void {
  const handlerRef = useRef(handler);

  useEffect(() => {
    handlerRef.current = handler;
  }, [handler]);

  useEffect(() => {
    const parsed = parseCombo(combo);
    const onKeyDown = (e: KeyboardEvent) => {
      if (matchesCombo(e, parsed)) {
        e.preventDefault();
        handlerRef.current();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [combo]);
}
