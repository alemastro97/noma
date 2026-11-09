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
import { Minus, Plus } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState, type MouseEvent } from "react";
import { useBoardStore } from "../../stores/board";
import { useUiStore } from "../../stores/ui";
import { progressFor, useCardHandlers, useNow, useWorkspaceData } from "../useWorkspace";
import styles from "./Canvas.module.css";
import { CardNode, type CardNodeType } from "./CardNode";
import { FlowEdge } from "./FlowEdge";
import { autoLayout, boardLayout } from "./layout";

const nodeTypes = { card: CardNode };
const edgeTypes = { flow: FlowEdge };
const PRO_OPTIONS = { hideAttribution: true };

interface CanvasViewProps {
  /** Board shown, when the target is a project or personal board. Filters have none. */
  boardId: string | null;
  /** Changes when the user navigates; the view re-fits on a new key. */
  viewKey: string;
  empty?: React.ReactNode;
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

function Canvas({ boardId, viewKey, empty }: CanvasViewProps) {
  const { items, projectsById, progress, freshId, selectedId } = useWorkspaceData();
  const positions = useBoardStore((s) => s.positions);
  const links = useBoardStore((s) => s.links);
  const handlers = useCardHandlers();
  const now = useNow();
  const { fitView, screenToFlowPosition } = useReactFlow();
  const [selectedEdge, setSelectedEdge] = useState<string | null>(null);
  const isBoard = boardId !== null;

  const layout = useMemo(
    () => (isBoard ? boardLayout(items, positions) : autoLayout(items)),
    [isBoard, items, positions],
  );

  const built = useMemo<CardNodeType[]>(
    () =>
      items.map((item) => ({
        id: item.id,
        type: "card",
        position: layout[item.id] ?? { x: 0, y: 0 },
        selected: item.id === selectedId,
        data: {
          item,
          project: item.projectId ? projectsById[item.projectId] : undefined,
          progress: progressFor(item, progress),
          now,
          fresh: item.id === freshId,
          isSelected: item.id === selectedId,
          connectable: isBoard,
          handlers,
        },
      })),
    [items, layout, selectedId, projectsById, progress, now, freshId, isBoard, handlers],
  );

  const [nodes, setNodes, onNodesChange] = useNodesState<CardNodeType>(built);
  useEffect(() => setNodes(built), [built, setNodes]);

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

  const onNodeDragStop: OnNodeDrag<CardNodeType> = useCallback(
    (_, node) => {
      if (isBoard) void useBoardStore.getState().move(node.id, node.position);
    },
    [isBoard],
  );

  const onNodeClick: NodeMouseHandler<CardNodeType> = useCallback((_, node) => {
    setSelectedEdge(null);
    useUiStore.getState().select(node.id);
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

  return (
    <div className={styles.canvas} onDoubleClick={onPaneDoubleClick}>
      <ReactFlow<CardNodeType>
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onNodesChange={onNodesChange}
        onNodeDragStop={onNodeDragStop}
        onNodeClick={onNodeClick}
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
      <ZoomPill />
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
