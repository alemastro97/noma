import type { Item, ItemType } from "../../api/types";
import type { Point } from "../../stores/board";

/** Rough rendered heights per type, used to stack cards in the automatic layout. */
const EST_HEIGHT: Record<ItemType, number> = {
  task: 110,
  reminder: 150,
  note: 210,
  idea: 150,
  project_ref: 170,
  event: 100,
};

export const AUTO_COLUMNS = 4;
export const COLUMN_WIDTH = 320;
export const ROW_GAP = 28;
export const ORIGIN = 80;

/**
 * Masonry placement for canvases that are filters rather than boards (Inbox,
 * Today…): each card goes into the currently shortest column. Not persisted.
 */
export function autoLayout(items: Item[], columns = AUTO_COLUMNS): Record<string, Point> {
  const heights = new Array<number>(columns).fill(ORIGIN);
  const out: Record<string, Point> = {};
  for (const item of items) {
    let col = 0;
    for (let c = 1; c < columns; c++) if (heights[c]! < heights[col]!) col = c;
    out[item.id] = { x: ORIGIN + col * COLUMN_WIDTH, y: heights[col]! };
    heights[col]! += EST_HEIGHT[item.kind] + ROW_GAP;
  }
  return out;
}

/** Board positions, with a fallback slot for any card that has none yet. */
export function boardLayout(
  items: Item[],
  positions: Record<string, Point>,
): Record<string, Point> {
  const missing = items.filter((i) => !positions[i.id]);
  if (missing.length === 0) return positions;
  const maxY = Math.max(ORIGIN, ...Object.values(positions).map((p) => p.y + 220));
  const extra = autoLayout(missing);
  const shifted: Record<string, Point> = { ...positions };
  for (const [id, p] of Object.entries(extra)) shifted[id] = { x: p.x, y: p.y - ORIGIN + maxY };
  return shifted;
}
