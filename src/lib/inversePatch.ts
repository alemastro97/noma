import type { Item, ItemPatch } from "../api/types";

/** The patch that puts back what `patch` is about to change on `before`. */
export function inversePatch(before: Item, patch: ItemPatch): ItemPatch {
  const out: Record<string, unknown> = {};
  for (const key of Object.keys(patch) as (keyof ItemPatch)[]) {
    if (patch[key] === undefined) continue;
    const prev = before[key as keyof Item];
    out[key] = Array.isArray(prev) ? [...prev] : (prev ?? null);
  }
  return out as ItemPatch;
}
