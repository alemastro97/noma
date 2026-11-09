import { copy } from "../../copy";
import { NomaOrb } from "../../components/NomaOrb";
import styles from "./orb.module.css";

/** Content of the small transparent always-on-top `orb` window. */
export function OrbWindow() {
  return (
    <div className={styles.root} data-tauri-drag-region>
      <NomaOrb size={52} label={copy.orbHere} className={styles.orb} />
    </div>
  );
}
