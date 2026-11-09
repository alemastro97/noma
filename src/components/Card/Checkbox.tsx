import { Check } from "lucide-react";
import { cardCopy } from "../../copy";
import styles from "./Card.module.css";

interface CheckboxProps {
  checked: boolean;
  /** Ring color while unchecked (token var). */
  color?: string;
  size?: number;
  onToggle?: () => void;
}

export function Checkbox({
  checked,
  color = "var(--noma-lavender)",
  size = 17,
  onToggle,
}: CheckboxProps) {
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={checked}
      aria-label={checked ? cardCopy.reopen : cardCopy.complete}
      className={styles.checkbox}
      data-checked={checked}
      style={{ width: size, height: size, borderColor: checked ? undefined : color }}
      onClick={(e) => {
        e.stopPropagation();
        onToggle?.();
      }}
      onDoubleClick={(e) => e.stopPropagation()}
    >
      {checked && <Check size={size - 6} strokeWidth={3} aria-hidden="true" />}
    </button>
  );
}
