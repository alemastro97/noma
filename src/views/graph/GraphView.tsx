import { useEffect, useMemo, useRef, useState, type PointerEvent, type ReactNode } from "react";
import type { Link } from "../../api/types";
import { listAllLinks } from "../../api/workspace";
import { KIND_STYLE } from "../../components/Card/kind";
import { EmptyState } from "../../components/EmptyState/EmptyState";
import { workspaceCopy } from "../../copy";
import { seedPosition, tick, type SimNode } from "../../lib/force";
import { selectCard } from "../../stores/selection";
import { useUiStore } from "../../stores/ui";
import { useWorkspaceData } from "../useWorkspace";
import styles from "./Graph.module.css";

const MAX_LABEL = 28;
const MIN_ZOOM = 0.3;
const MAX_ZOOM = 2.5;

function shorten(s: string): string {
  return s.length > MAX_LABEL ? `${s.slice(0, MAX_LABEL - 1)}…` : s;
}

interface View {
  x: number;
  y: number;
  k: number;
}

type Drag =
  | { kind: "pan"; startX: number; startY: number; view: View; moved: boolean }
  | { kind: "node"; id: string; startX: number; startY: number; moved: boolean };

/** Cards as dots and their links as lines, laid out by a small force simulation. */
export function GraphView({ empty }: { empty?: ReactNode }) {
  const { items, selection } = useWorkspaceData();
  const [links, setLinks] = useState<Link[]>([]);
  const [onlyLinked, setOnlyLinked] = useState(false);
  const [, setFrame] = useState(0);
  const [view, setView] = useState<View>({ x: 0, y: 0, k: 1 });
  const svgRef = useRef<SVGSVGElement>(null);
  // Positions survive data refreshes (a stable map, mutated by the simulation).
  const [sim] = useState(() => new Map<string, SimNode>());
  const alpha = useRef(1);
  const raf = useRef<number | null>(null);
  const drag = useRef<Drag | null>(null);
  const lastClick = useRef<{ id: string; at: number } | null>(null);

  // Links change with the cards' link counts; refetch only then.
  const linkKey = useMemo(() => items.map((i) => `${i.id}:${i.linkCount}`).join(","), [items]);
  useEffect(() => {
    let live = true;
    listAllLinks()
      .then((l) => live && setLinks(l ?? []))
      .catch(() => live && setLinks([]));
    return () => {
      live = false;
    };
  }, [linkKey]);

  const { nodes, edges, shown } = useMemo(() => {
    const present = new Set(items.map((i) => i.id));
    const visibleLinks = links.filter((l) => present.has(l.fromItem) && present.has(l.toItem));
    const linked = new Set(visibleLinks.flatMap((l) => [l.fromItem, l.toItem]));
    const shown = onlyLinked ? items.filter((i) => linked.has(i.id)) : items;
    const map = sim;
    const nodes = shown.map((item, index) => {
      let n = map.get(item.id);
      if (!n) {
        n = { id: item.id, ...seedPosition(index), vx: 0, vy: 0 };
        map.set(item.id, n);
      }
      return n;
    });
    const index = new Map(nodes.map((n, i) => [n.id, i]));
    const edges = visibleLinks
      .map((l) => [index.get(l.fromItem), index.get(l.toItem)] as const)
      .filter((e): e is [number, number] => e[0] !== undefined && e[1] !== undefined);
    return { nodes, edges, shown };
  }, [items, links, onlyLinked, sim]);

  // Run the simulation until it settles; restart whenever the graph changes.
  const kick = (to = 0.8) => {
    alpha.current = Math.max(alpha.current, to);
    if (raf.current !== null) return;
    const loop = () => {
      const moved = tick(nodes, edges, alpha.current);
      alpha.current *= 0.985;
      setFrame((f) => f + 1);
      if (alpha.current > 0.02 && moved > 0.05) raf.current = requestAnimationFrame(loop);
      else raf.current = null;
    };
    raf.current = requestAnimationFrame(loop);
  };
  useEffect(() => {
    if (raf.current !== null) cancelAnimationFrame(raf.current);
    raf.current = null;
    kick(0.8);
    return () => {
      if (raf.current !== null) cancelAnimationFrame(raf.current);
      raf.current = null;
    };
    // `kick` closes over the current nodes/edges.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [nodes, edges]);

  // Center the origin once the size is known.
  useEffect(() => {
    const el = svgRef.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    setView((v) => ({ ...v, x: r.width / 2, y: r.height / 2 }));
  }, []);

  const toWorld = (clientX: number, clientY: number) => {
    const r = svgRef.current?.getBoundingClientRect();
    return {
      x: (clientX - (r?.left ?? 0) - view.x) / view.k,
      y: (clientY - (r?.top ?? 0) - view.y) / view.k,
    };
  };

  const onPointerDown = (e: PointerEvent<SVGSVGElement>) => {
    const host = (e.target as Element).closest<SVGGElement>("[data-item-id]");
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
    if (host?.dataset.itemId) {
      drag.current = {
        kind: "node",
        id: host.dataset.itemId,
        startX: e.clientX,
        startY: e.clientY,
        moved: false,
      };
    } else {
      drag.current = { kind: "pan", startX: e.clientX, startY: e.clientY, view, moved: false };
    }
  };

  const onPointerMove = (e: PointerEvent<SVGSVGElement>) => {
    const d = drag.current;
    if (!d) return;
    if (Math.abs(e.clientX - d.startX) + Math.abs(e.clientY - d.startY) > 3) d.moved = true;
    if (!d.moved) return;
    if (d.kind === "pan") {
      setView({
        ...d.view,
        x: d.view.x + e.clientX - d.startX,
        y: d.view.y + e.clientY - d.startY,
      });
    } else {
      const n = sim.get(d.id);
      if (!n) return;
      const p = toWorld(e.clientX, e.clientY);
      n.fixed = true;
      n.x = p.x;
      n.y = p.y;
      kick(0.3);
    }
  };

  const onPointerUp = (e: PointerEvent<SVGSVGElement>) => {
    const d = drag.current;
    drag.current = null;
    if (!d) return;
    if (d.kind === "node") {
      const n = sim.get(d.id);
      if (n) n.fixed = false;
      if (!d.moved) {
        // Pointer capture retargets dblclick, so detect the second click here.
        const now = Date.now();
        const last = lastClick.current;
        lastClick.current = { id: d.id, at: now };
        if (last && last.id === d.id && now - last.at < 350) {
          useUiStore.getState().openDetail(d.id);
          return;
        }
        selectCard(d.id, { toggle: e.metaKey || e.ctrlKey, range: e.shiftKey });
      }
    } else if (!d.moved) {
      useUiStore.getState().select(null);
    }
  };

  const onWheel = (e: React.WheelEvent<SVGSVGElement>) => {
    const r = svgRef.current?.getBoundingClientRect();
    const px = e.clientX - (r?.left ?? 0);
    const py = e.clientY - (r?.top ?? 0);
    setView((v) => {
      const k = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, v.k * Math.exp(-e.deltaY * 0.0015)));
      return { k, x: px - ((px - v.x) * k) / v.k, y: py - ((py - v.y) * k) / v.k };
    });
  };

  const byId = useMemo(() => new Map(shown.map((i) => [i.id, i])), [shown]);

  return (
    <div className={styles.graph}>
      <svg
        ref={svgRef}
        className={styles.svg}
        role="img"
        aria-label={workspaceCopy.viewGraph}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onWheel={onWheel}
      >
        <g transform={`translate(${view.x} ${view.y}) scale(${view.k})`}>
          {edges.map(([s, t], i) => {
            const a = nodes[s]!;
            const b = nodes[t]!;
            const hot = selection.has(a.id) || selection.has(b.id);
            return (
              <line
                key={i}
                className={styles.edge}
                data-hot={hot || undefined}
                x1={a.x}
                y1={a.y}
                x2={b.x}
                y2={b.y}
              />
            );
          })}
          {nodes.map((n) => {
            const item = byId.get(n.id);
            if (!item) return null;
            const r = 7 + Math.min(item.linkCount, 6) * 1.5;
            return (
              <g
                key={n.id}
                className={styles.node}
                data-item-id={n.id}
                data-selected={selection.has(n.id) || undefined}
                data-done={item.status === "done" || undefined}
                transform={`translate(${n.x} ${n.y})`}
              >
                <title>{item.title}</title>
                <circle className={styles.ring} r={r + 4} />
                <circle r={r} style={{ fill: KIND_STYLE[item.kind].color }} />
                <text className={styles.label} y={r + 14}>
                  {shorten(item.title)}
                </text>
              </g>
            );
          })}
        </g>
      </svg>
      <div className={styles.tools}>
        <button
          type="button"
          className={styles.chip}
          aria-pressed={onlyLinked}
          onClick={() => setOnlyLinked((v) => !v)}
        >
          {workspaceCopy.onlyLinked}
        </button>
      </div>
      {nodes.length === 0 && (
        <div className={styles.empty}>
          {items.length === 0 ? (
            empty
          ) : (
            <EmptyState
              title={workspaceCopy.graphEmpty.title}
              body={workspaceCopy.graphEmpty.body}
              variant="card"
            />
          )}
        </div>
      )}
    </div>
  );
}
