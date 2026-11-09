import { copy } from "../../copy";
import { NomaOrb, type OrbState } from "../NomaOrb";
import styles from "./Header.module.css";

const LABEL: Record<OrbState, string> = {
  idle: copy.aiIdle,
  listening: copy.aiListening,
  processing: copy.aiProcessing,
  success: copy.aiSuccess,
  error: copy.aiError,
};

export function AiStatusChip({ state = "idle" }: { state?: OrbState }) {
  return (
    <div className={styles.aiChip}>
      <NomaOrb size={16} state={state} label={`${copy.aiStatus}: ${LABEL[state]}`} />
      <span>{LABEL[state]}</span>
    </div>
  );
}
