import { NodeResizer, type Node, type NodeProps } from "@xyflow/react";
import { X } from "lucide-react";
import { memo, useEffect, useRef, useState } from "react";
import type { CanvasGroup } from "../../api/workspace";
import { workspaceCopy } from "../../copy";
import styles from "./Frame.module.css";

/** Drag handle selector (a global class: CSS-module names are hashed). */
export const FRAME_HANDLE = "noma-frame-handle";

export type FrameNodeData = {
  group: CanvasGroup;
  /** Starts with the label in edit mode (a freshly drawn frame). */
  editing: boolean;
  onRename: (id: string, label: string) => void;
  onDelete: (id: string) => void;
  onResized: (id: string, rect: { x: number; y: number; w: number; h: number }) => void;
};

export type FrameNodeType = Node<FrameNodeData, "frame">;

export const FrameNode = memo(function FrameNode({ data }: NodeProps<FrameNodeType>) {
  const { group, editing, onRename, onDelete, onResized } = data;
  const [edit, setEdit] = useState(editing);
  const [label, setLabel] = useState(group.label);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => setLabel(group.label), [group.label]);
  useEffect(() => {
    if (edit) input.current?.select();
  }, [edit]);

  const commit = () => {
    setEdit(false);
    const value = label.trim() || workspaceCopy.groupDefault;
    if (value !== group.label) onRename(group.id, value);
    else setLabel(group.label);
  };

  return (
    <div className={styles.frame}>
      <NodeResizer
        minWidth={120}
        minHeight={100}
        lineClassName={styles.resizeLine}
        handleClassName={styles.resizeHandle}
        onResizeEnd={(_, p) => onResized(group.id, { x: p.x, y: p.y, w: p.width, h: p.height })}
      />
      <div className={`${styles.header} ${FRAME_HANDLE}`}>
        {edit ? (
          <input
            ref={input}
            className={`${styles.input} nodrag`}
            aria-label={workspaceCopy.groupRename}
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            onBlur={commit}
            onKeyDown={(e) => {
              e.stopPropagation();
              if (e.key === "Enter") commit();
              else if (e.key === "Escape") {
                setLabel(group.label);
                setEdit(false);
              }
            }}
          />
        ) : (
          <span
            className={styles.label}
            title={workspaceCopy.groupRename}
            role="button"
            tabIndex={0}
            aria-label={`${workspaceCopy.groupRename}: ${group.label}`}
            onDoubleClick={(e) => {
              e.stopPropagation();
              setEdit(true);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === "F2") {
                e.preventDefault();
                e.stopPropagation();
                setEdit(true);
              }
            }}
          >
            {group.label}
          </span>
        )}
        <button
          type="button"
          className={`${styles.remove} nodrag`}
          aria-label={workspaceCopy.groupDelete}
          title={workspaceCopy.groupDelete}
          onClick={(e) => {
            e.stopPropagation();
            onDelete(group.id);
          }}
        >
          <X size={12} strokeWidth={1.75} aria-hidden="true" />
        </button>
      </div>
    </div>
  );
});
