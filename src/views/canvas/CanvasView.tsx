import {
  Background,
  BackgroundVariant,
  ReactFlow,
  ReactFlowProvider,
  useNodesState,
  useReactFlow,
  useViewport,
  type Connection,
  type Edge,
  type NodeMouseHandler,
  type OnNodeDrag,
} from "@xyflow/react";
import "@xyflow/react/dist/base.css";
import { SquareDashed, Minus, Plus } from "lucide-react";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
  type PointerEvent,
} from "react";
import { workspaceCopy } from "../../copy";
import { clearHover, hoverAtPoint, placeAtPoint } from "../../components/Sidebar/dropTarget";
import { useBoardStore, type Point } from "../../stores/board";
import { moveCards } from "../../stores/bulk";
import { insideRect, useGroupsStore } from "../../stores/groups";
import { record } from "../../stores/history";
import { useSelectionStore } from "../../stores/selection";
import { useUiStore } from "../../stores/ui";
import { progressFor, useCardHandlers, useNow, useWorkspaceData } from "../useWorkspace";
import styles from "./Canvas.module.css";
import { CardNode, type CardNodeType } from "./CardNode";
import frameStyles from "./Frame.module.css";
import { FRAME_HANDLE, FrameNode, type FrameNodeData, type FrameNodeType } from "./FrameNode";
import { FlowEdge } from "./FlowEdge";
import { autoLayout, boardLayout } from "./layout";

type AnyNode = CardNodeType | FrameNodeType;

const nodeTypes = { card: CardNode, frame: FrameNode };
const edgeTypes = { flow: FlowEdge };
const PRO_OPTIONS = { hideAttribution: true };
const FRAME_PREFIX = "frame:";
/** Card box used for "inside a group" before xyflow has measured it. */
const CARD_W = 260;
const CARD_H = 150;

interface CanvasViewProps {
  /** Board shown, when the target is a project or personal board. Filters have none. */
  boardId: string | null;
  /** Changes when the user navigates; the view re-fits on a new key. */
  viewKey: string;
  empty?: React.ReactNode;
}

interface FrameDrag {
  groupId: string;
  start: Point;
  cards: Map<string, Point>;
}

function ZoomPill() {
  const { zoomIn, zoomOut } = useReactFlow();
  const { zoom } = useViewport();
  return (
    <div className={styles.zoom}>
      <button
        type="button"
        className={styles.zoomBtn}
        aria-label="Zoom out"
        onClick={() => void zoomOut({ duration: 200 })}
      >
        <Minus size={15} strokeWidth={1.75} aria-hidden="true" />
      </button>
      <span className={styles.zoomValue}>{Math.round(zoom * 100)}%</span>
      <button
        type="button"
        className={styles.zoomBtn}
        aria-label="Zoom in"
        onClick={() => void zoomIn({ duration: 200 })}
      >
        <Plus size={15} strokeWidth={1.75} aria-hidden="true" />
      </button>
    </div>
  );
}

function clientPoint(
  e: { clientX?: number; clientY?: number } | TouchEvent | globalThis.MouseEvent,
) {
  if ("clientX" in e && typeof e.clientX === "number")
    return { x: e.clientX, y: (e as { clientY: number }).clientY };
  const t = (e as TouchEvent).changedTouches?.[0];
  return t ? { x: t.clientX, y: t.clientY } : null;
}

function movePositions(points: Map<string, Point>) {
  const board = useBoardStore.getState();
  return Promise.all([...points].map(([id, p]) => board.move(id, p)));
}

function Canvas({ boardId, viewKey, empty }: CanvasViewProps) {
  const { items, projectsById, progress, freshId, selection } = useWorkspaceData();
  const positions = useBoardStore((s) => s.positions);
  const links = useBoardStore((s) => s.links);
  const groups = useGroupsStore((s) => s.groups);
  const handlers = useCardHandlers();
  const now = useNow();
  const { fitView, screenToFlowPosition, getNodes } = useReactFlow<AnyNode>();
  const [selectedEdge, setSelectedEdge] = useState<string | null>(null);
  const [drawing, setDrawing] = useState(false);
  const [draft, setDraft] = useState<{ x0: number; y0: number; x1: number; y1: number } | null>(
    null,
  );
  const [editingGroup, setEditingGroup] = useState<string | null>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const dragStart = useRef<Map<string, Point>>(new Map());
  const frameDrag = useRef<FrameDrag | null>(null);
  const isBoard = boardId !== null;

  useEffect(() => {
    void useGroupsStore.getState().load(boardId);
  }, [boardId]);

  const layout = useMemo(
    () => (isBoard ? boardLayout(items, positions) : autoLayout(items)),
    [isBoard, items, positions],
  );

  const frameHandlers = useMemo<Omit<FrameNodeData, "group" | "editing">>(
    () => ({
      onRename: (id, label) => {
        setEditingGroup(null);
        void useGroupsStore.getState().update(id, { label });
      },
      onDelete: (id) => void useGroupsStore.getState().remove(id),
      onResized: (id, rect) => void useGroupsStore.getState().update(id, rect),
    }),
    [],
  );

  const built = useMemo<AnyNode[]>(() => {
    const frames: FrameNodeType[] = isBoard
      ? groups.map((g) => ({
          id: `${FRAME_PREFIX}${g.id}`,
          type: "frame",
          position: { x: g.x, y: g.y },
          width: g.w,
          height: g.h,
          zIndex: -1,
          selectable: false,
          dragHandle: `.${FRAME_HANDLE}`,
          data: { group: g, editing: g.id === editingGroup, ...frameHandlers },
        }))
      : [];
    const cards: CardNodeType[] = items.map((item) => ({
      id: item.id,
      type: "card",
      position: layout[item.id] ?? { x: 0, y: 0 },
      selected: selection.has(item.id),
      data: {
        item,
        project: item.projectId ? projectsById[item.projectId] : undefined,
        progress: progressFor(item, progress),
        now,
        fresh: item.id === freshId,
        isSelected: selection.has(item.id),
        connectable: isBoard,
        handlers,
      },
    }));
    return [...frames, ...cards];
  }, [
    isBoard,
    groups,
    editingGroup,
    frameHandlers,
    items,
    layout,
    selection,
    projectsById,
    progress,
    now,
    freshId,
    handlers,
  ]);

  const [nodes, setNodes, onNodesChange] = useNodesState<AnyNode>(built);
  // Keep xyflow's measured sizes across rebuilds: a node without `measured`
  // stays hidden until its ResizeObserver fires again, which never happens
  // when the card's size did not change. Frames keep their stored size.
  useEffect(
    () =>
      setNodes((prev) => {
        const old = new Map(prev.map((n) => [n.id, n]));
        return built.map((n) => {
          const o = old.get(n.id);
          if (!o?.measured) return n;
          if (n.type === "frame") return { ...n, measured: o.measured } as AnyNode;
          return { ...n, measured: o.measured, width: o.width, height: o.height } as AnyNode;
        });
      }),
    [built, setNodes],
  );

  const edges = useMemo<Edge[]>(() => {
    const present = new Set(items.map((i) => i.id));
    return links
      .filter((l) => present.has(l.fromItem) && present.has(l.toItem))
      .map((l) => ({
        id: l.id,
        source: l.fromItem,
        target: l.toItem,
        type: "flow",
        selected: l.id === selectedEdge,
      }));
  }, [links, items, selectedEdge]);

  // Fit once per navigation, as soon as there is something to fit.
  const fittedFor = useRef<string | null>(null);
  useEffect(() => {
    if (fittedFor.current === viewKey || built.length === 0) return;
    fittedFor.current = viewKey;
    requestAnimationFrame(() => void fitView({ padding: 0.15, maxZoom: 1, duration: 0 }));
  }, [viewKey, built.length, fitView]);

  const onNodeDragStart: OnNodeDrag<AnyNode> = useCallback(
    (_, node, dragged) => {
      dragStart.current = new Map(dragged.map((n) => [n.id, { ...n.position }]));
      if (node.type !== "frame") return;
      const g = node.data.group;
      const rect = { x: node.position.x, y: node.position.y, w: g.w, h: g.h };
      const cards = getNodes()
        .filter((n) => n.type === "card")
        .map((n) => ({
          id: n.id,
          x: n.position.x,
          y: n.position.y,
          w: n.measured?.width ?? CARD_W,
          h: n.measured?.height ?? CARD_H,
        }));
      const inside = new Set(insideRect(rect, cards));
      frameDrag.current = {
        groupId: g.id,
        start: { ...node.position },
        cards: new Map(
          cards.filter((c) => inside.has(c.id)).map((c) => [c.id, { x: c.x, y: c.y }]),
        ),
      };
    },
    [getNodes],
  );

  const onNodeDrag: OnNodeDrag<AnyNode> = useCallback(
    (event, node) => {
      const fd = frameDrag.current;
      if (node.type === "frame" && fd) {
        const dx = node.position.x - fd.start.x;
        const dy = node.position.y - fd.start.y;
        setNodes((nds) =>
          nds.map((n) => {
            const p = fd.cards.get(n.id);
            return p ? { ...n, position: { x: p.x + dx, y: p.y + dy } } : n;
          }),
        );
        return;
      }
      const at = clientPoint(event);
      if (at) hoverAtPoint(at.x, at.y);
    },
    [setNodes],
  );

  const onNodeDragStop: OnNodeDrag<AnyNode> = useCallback(
    (event, node, dragged) => {
      clearHover();
      const fd = frameDrag.current;
      frameDrag.current = null;

      if (node.type === "frame" && fd) {
        const to = { ...node.position };
        const dx = to.x - fd.start.x;
        const dy = to.y - fd.start.y;
        if (dx === 0 && dy === 0) return;
        const cardsTo = new Map(
          [...fd.cards].map(([id, p]) => [id, { x: p.x + dx, y: p.y + dy }] as const),
        );
        const apply = (frame: Point, cards: Map<string, Point>) => {
          void useGroupsStore.getState().update(fd.groupId, frame, { record: false });
          return movePositions(cards);
        };
        void apply(to, cardsTo);
        record({
          action: "group",
          undo: () => apply(fd.start, fd.cards),
          redo: () => apply(to, cardsTo),
        });
        return;
      }

      const cards = dragged.filter((n) => n.type === "card");
      if (cards.length === 0) return;
      const at = clientPoint(event);
      const drop = at ? placeAtPoint(at.x, at.y) : null;
      if (drop) {
        void moveCards(
          cards.map((n) => n.id),
          drop.place,
          drop.name,
        );
        return;
      }
      if (!isBoard) return;
      const from = new Map<string, Point>();
      const to = new Map<string, Point>();
      for (const n of cards) {
        const start = dragStart.current.get(n.id);
        if (start && (start.x !== n.position.x || start.y !== n.position.y)) {
          from.set(n.id, start);
          to.set(n.id, { ...n.position });
        }
      }
      if (to.size === 0) return;
      void movePositions(to);
      record({
        action: "position",
        undo: () => movePositions(from),
        redo: () => movePositions(to),
      });
    },
    [isBoard],
  );

  const onNodeClick: NodeMouseHandler<AnyNode> = useCallback(() => {
    // The card itself selects on click (with ⌘ / ⇧); here only the edge is cleared.
    setSelectedEdge(null);
  }, []);

  const onConnect = useCallback(
    (c: Connection) => {
      if (isBoard && c.source && c.target)
        void useBoardStore.getState().connect(c.source, c.target);
    },
    [isBoard],
  );

  const onPaneDoubleClick = useCallback(
    (e: MouseEvent) => {
      const target = e.target as Element;
      if (!target.classList.contains("react-flow__pane")) return;
      const position = screenToFlowPosition({ x: e.clientX, y: e.clientY });
      useUiStore.getState().openQuickAdd(isBoard ? { position } : {});
    },
    [screenToFlowPosition, isBoard],
  );

  // Shift-drag box selection on the pane feeds the shared multi-select.
  const onSelectionEnd = useCallback(() => {
    const ids = getNodes()
      .filter((n) => n.type === "card" && n.selected)
      .map((n) => n.id);
    if (ids.length === 0) return;
    useSelectionStore.getState().set(ids, ids[0]);
    useUiStore.getState().select(ids[ids.length - 1]!);
  }, [getNodes]);

  // Backspace removes a selected connection (cards are handled by the global hotkeys).
  useEffect(() => {
    if (!selectedEdge) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Backspace" || e.key === "Delete") {
        e.preventDefault();
        void useBoardStore.getState().disconnect(selectedEdge);
        setSelectedEdge(null);
      } else if (e.key === "Escape") setSelectedEdge(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [selectedEdge]);

  // Esc leaves the group tool.
  useEffect(() => {
    if (!drawing) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopImmediatePropagation();
        setDrawing(false);
        setDraft(null);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [drawing]);

  const local = (e: PointerEvent) => {
    const r = wrapRef.current?.getBoundingClientRect();
    return { x: e.clientX - (r?.left ?? 0), y: e.clientY - (r?.top ?? 0) };
  };

  const onDrawEnd = async (e: PointerEvent) => {
    const d = draft;
    setDraft(null);
    setDrawing(false);
    if (!d) return;
    const r = wrapRef.current?.getBoundingClientRect();
    const ox = r?.left ?? 0;
    const oy = r?.top ?? 0;
    const a = screenToFlowPosition({ x: Math.min(d.x0, d.x1) + ox, y: Math.min(d.y0, d.y1) + oy });
    const b = screenToFlowPosition({ x: Math.max(d.x0, d.x1) + ox, y: Math.max(d.y0, d.y1) + oy });
    (e.target as Element).releasePointerCapture?.(e.pointerId);
    if (b.x - a.x < 40 || b.y - a.y < 40) return;
    const g = await useGroupsStore
      .getState()
      .create({ x: a.x, y: a.y, w: b.x - a.x, h: b.y - a.y }, workspaceCopy.groupDefault);
    if (g) setEditingGroup(g.id);
  };

  return (
    <div ref={wrapRef} className={styles.canvas} onDoubleClick={onPaneDoubleClick}>
      <ReactFlow<AnyNode>
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onNodesChange={onNodesChange}
        onNodeDragStart={onNodeDragStart}
        onNodeDrag={onNodeDrag}
        onNodeDragStop={onNodeDragStop}
        onNodeClick={onNodeClick}
        onSelectionEnd={onSelectionEnd}
        onEdgeClick={(_, edge) => {
          setSelectedEdge(edge.id);
          useUiStore.getState().select(null);
        }}
        onPaneClick={() => {
          setSelectedEdge(null);
          useUiStore.getState().select(null);
        }}
        onConnect={onConnect}
        nodesConnectable={isBoard}
        elementsSelectable
        selectNodesOnDrag={false}
        multiSelectionKeyCode={null}
        deleteKeyCode={null}
        zoomOnDoubleClick={false}
        minZoom={0.25}
        maxZoom={2}
        onlyRenderVisibleElements
        proOptions={PRO_OPTIONS}
        defaultViewport={{ x: 0, y: 0, zoom: 1 }}
      >
        <Background
          variant={BackgroundVariant.Dots}
          gap={24}
          size={1.2}
          color="var(--noma-surface-elevated)"
        />
      </ReactFlow>
      {drawing && isBoard && (
        <div
          className={frameStyles.drawLayer}
          title={workspaceCopy.groupToolHint}
          onPointerDown={(e) => {
            (e.target as Element).setPointerCapture(e.pointerId);
            const p = local(e);
            setDraft({ x0: p.x, y0: p.y, x1: p.x, y1: p.y });
          }}
          onPointerMove={(e) => {
            if (!draft) return;
            const p = local(e);
            setDraft({ ...draft, x1: p.x, y1: p.y });
          }}
          onPointerUp={(e) => void onDrawEnd(e)}
        >
          {draft && (
            <div
              className={frameStyles.draft}
              style={{
                left: Math.min(draft.x0, draft.x1),
                top: Math.min(draft.y0, draft.y1),
                width: Math.abs(draft.x1 - draft.x0),
                height: Math.abs(draft.y1 - draft.y0),
              }}
            />
          )}
        </div>
      )}
      <ZoomPill />
      {isBoard && (
        <button
          type="button"
          className={frameStyles.tool}
          aria-pressed={drawing}
          title={workspaceCopy.groupToolHint}
          onClick={() => setDrawing((v) => !v)}
        >
          <SquareDashed size={15} strokeWidth={1.75} aria-hidden="true" />
          {workspaceCopy.groupTool}
        </button>
      )}
      {items.length === 0 && empty && <div className={styles.empty}>{empty}</div>}
    </div>
  );
}

export function CanvasView(props: CanvasViewProps) {
  return (
    <ReactFlowProvider>
      <Canvas {...props} />
    </ReactFlowProvider>
  );
}
