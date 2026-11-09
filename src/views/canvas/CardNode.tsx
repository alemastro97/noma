import { Handle, Position, type Node, type NodeProps } from "@xyflow/react";
import { memo } from "react";
import type { Item, Progress, Project } from "../../api/types";
import { Card } from "../../components/Card/Card";
import type { CardHandlers } from "../useWorkspace";
import styles from "./Canvas.module.css";

export type CardNodeData = {
  item: Item;
  project?: Project;
  progress?: Progress;
  now: Date;
  fresh: boolean;
  isSelected: boolean;
  connectable: boolean;
  handlers: CardHandlers;
};

export type CardNodeType = Node<CardNodeData, "card">;

export const CardNode = memo(function CardNode({ data }: NodeProps<CardNodeType>) {
  const { item, project, progress, now, fresh, isSelected, connectable, handlers } = data;
  return (
    <div className={styles.node}>
      {connectable && <Handle type="target" position={Position.Left} className={styles.handle} />}
      <Card
        item={item}
        project={project}
        progress={progress}
        now={now}
        fresh={fresh}
        selected={isSelected}
        variant="canvas"
        {...handlers}
      />
      {connectable && <Handle type="source" position={Position.Right} className={styles.handle} />}
    </div>
  );
});
