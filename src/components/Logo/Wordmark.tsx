import { copy } from "../../copy";
import styles from "./Wordmark.module.css";

export function Wordmark({ className }: { className?: string }) {
  return (
    <span className={[styles.wordmark, className].filter(Boolean).join(" ")}>{copy.appName}</span>
  );
}
