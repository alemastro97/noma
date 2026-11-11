/**
 * A tiny force-directed layout (no dependency): pairwise repulsion, springs
 * along links, a weak pull to the origin, velocity damping. O(n²) per tick,
 * which is fine for a personal graph of a few hundred cards.
 */
export interface SimNode {
  id: string;
  x: number;
  y: number;
  vx: number;
  vy: number;
  /** Pinned while the user drags it. */
  fixed?: boolean;
}

export interface ForceOptions {
  repulsion: number;
  springLength: number;
  springStrength: number;
  gravity: number;
  damping: number;
}

export const DEFAULT_FORCES: ForceOptions = {
  repulsion: 5200,
  springLength: 120,
  springStrength: 0.04,
  gravity: 0.012,
  damping: 0.82,
};

/** Seeds positions on a spiral so the first ticks don't explode. */
export function seedPosition(index: number): { x: number; y: number } {
  const angle = index * 2.399963; // golden angle
  const r = 40 * Math.sqrt(index + 1);
  return { x: Math.cos(angle) * r, y: Math.sin(angle) * r };
}

/**
 * Advances the simulation one tick, scaled by `alpha` (cooling 1 → 0).
 * Returns the total movement, so callers can stop once it settles.
 */
export function tick(
  nodes: SimNode[],
  edges: [number, number][],
  alpha: number,
  o: ForceOptions = DEFAULT_FORCES,
): number {
  const n = nodes.length;
  const fx = new Float64Array(n);
  const fy = new Float64Array(n);

  for (let i = 0; i < n; i++) {
    const a = nodes[i]!;
    for (let j = i + 1; j < n; j++) {
      const b = nodes[j]!;
      let dx = a.x - b.x;
      let dy = a.y - b.y;
      let d2 = dx * dx + dy * dy;
      if (d2 < 0.01) {
        // Coincident nodes: nudge apart deterministically.
        dx = (i - j) * 0.1;
        dy = 0.1;
        d2 = dx * dx + dy * dy;
      }
      const f = o.repulsion / d2;
      const d = Math.sqrt(d2);
      fx[i]! += (dx / d) * f;
      fy[i]! += (dy / d) * f;
      fx[j]! -= (dx / d) * f;
      fy[j]! -= (dy / d) * f;
    }
  }

  for (const [s, t] of edges) {
    const a = nodes[s];
    const b = nodes[t];
    if (!a || !b) continue;
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    const d = Math.sqrt(dx * dx + dy * dy) || 0.01;
    const f = (d - o.springLength) * o.springStrength;
    fx[s]! += (dx / d) * f;
    fy[s]! += (dy / d) * f;
    fx[t]! -= (dx / d) * f;
    fy[t]! -= (dy / d) * f;
  }

  let moved = 0;
  for (let i = 0; i < n; i++) {
    const p = nodes[i]!;
    if (p.fixed) {
      p.vx = 0;
      p.vy = 0;
      continue;
    }
    p.vx = (p.vx + (fx[i]! - p.x * o.gravity) * alpha) * o.damping;
    p.vy = (p.vy + (fy[i]! - p.y * o.gravity) * alpha) * o.damping;
    // Cap the step so a bad seed can't fling a node off screen.
    const speed = Math.hypot(p.vx, p.vy);
    if (speed > 40) {
      p.vx = (p.vx / speed) * 40;
      p.vy = (p.vy / speed) * 40;
    }
    p.x += p.vx;
    p.y += p.vy;
    moved += Math.abs(p.vx) + Math.abs(p.vy);
  }
  return moved;
}
