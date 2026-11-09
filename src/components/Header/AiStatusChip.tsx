import { copy } from "../../copy";
import { aiStatusLabel, useAiStore } from "../../stores/ai";
import { useUiStore } from "../../stores/ui";
import { NomaOrb } from "../NomaOrb";
import styles from "./Header.module.css";

/** Live AI status: orb state + short label. Click to open the AI settings. */
export function AiStatusChip() {
  const state = useAiStore((s) => s.state);
  const orb = useAiStore((s) => s.orb);
  const label = aiStatusLabel(state, orb);
  return (
    <button
      type="button"
      className={styles.aiChip}
      onClick={() => useUiStore.getState().setTarget({ kind: "settings" })}
      title={state?.message ?? label}
    >
      <NomaOrb size={16} state={orb} label={`${copy.aiStatus}: ${label}`} />
      <span>{label}</span>
    </button>
  );
}
