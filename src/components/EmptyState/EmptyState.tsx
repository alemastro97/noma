import { NomaOrb } from "../NomaOrb";
import styles from "./EmptyState.module.css";

interface EmptyStateProps {
  title: string;
  body: string;
  /** "card" draws the surface + border; "dashed" is the lighter project variant. */
  variant?: "card" | "dashed";
  orbSize?: number;
}

export function EmptyState({ title, body, variant = "card", orbSize = 44 }: EmptyStateProps) {
  return (
    <div className={styles.empty} data-variant={variant} role="status">
      <NomaOrb size={orbSize} label="" />
      <span className={styles.title}>{title}</span>
      <span className={styles.body}>{body}</span>
    </div>
  );
}
