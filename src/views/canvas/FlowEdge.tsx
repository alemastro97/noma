import { BaseEdge, getBezierPath, type EdgeProps } from "@xyflow/react";
import { memo } from "react";
import styles from "./Canvas.module.css";

/** Dashed, slowly flowing connection (design/screens/Workspace.html `.link`). */
export const FlowEdge = memo(function FlowEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  selected,
}: EdgeProps) {
  const [path] = getBezierPath({
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
  });
  return (
    <>
      <BaseEdge id={id} path={path} className={styles.edge} data-selected={selected || undefined} />
      <circle
        cx={targetX}
        cy={targetY}
        r={3.5}
        className={styles.edgeDot}
        data-selected={selected || undefined}
      />
    </>
  );
});
