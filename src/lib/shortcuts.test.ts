import {
  acceleratorFromEvent,
  acceleratorParts,
  formatAccelerator,
  heldModifiers,
  isComplete,
} from "./shortcuts";

const ev = (
  code: string,
  mods: Partial<Record<"alt" | "meta" | "ctrl" | "shift", boolean>> = {},
) => ({
  code,
  altKey: !!mods.alt,
  metaKey: !!mods.meta,
  ctrlKey: !!mods.ctrl,
  shiftKey: !!mods.shift,
});

test("key press becomes an accelerator in a stable modifier order", () => {
  expect(acceleratorFromEvent(ev("Space", { alt: true }))).toBe("Alt+Space");
  expect(acceleratorFromEvent(ev("KeyN", { meta: true, shift: true }))).toBe("Shift+Super+KeyN");
});

test("modifier-only presses have no accelerator yet but show live", () => {
  expect(acceleratorFromEvent(ev("AltLeft", { alt: true }))).toBeNull();
  expect(heldModifiers(ev("AltLeft", { alt: true, shift: true }))).toEqual(["Alt", "Shift"]);
});

test("accelerators render with mac symbols", () => {
  expect(formatAccelerator("Alt+Space")).toBe("⌥ Space");
  expect(formatAccelerator("Super+Shift+KeyN")).toBe("⌘⇧N");
  expect(acceleratorParts("Control+Digit1")).toEqual(["⌃", "1"]);
});

test("a shortcut needs a modifier and a key", () => {
  expect(isComplete("Alt+Space")).toBe(true);
  expect(isComplete("Space")).toBe(false);
  expect(isComplete("Alt")).toBe(false);
  expect(isComplete(null)).toBe(false);
});
