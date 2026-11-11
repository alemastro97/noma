import { Download, RefreshCw, Trash2, X } from "lucide-react";
import { useEffect, useState } from "react";
import {
  cancelSemanticDownload,
  configureSemantic,
  downloadSemanticModel,
  getSemanticState,
  onSemanticState,
  reindexSemantic,
  removeSemanticModel,
  type SemanticState,
} from "../../api/semantic";
import { semanticCopy as t, settingsCopy } from "../../copy";
import styles from "./Settings.module.css";

function statusLine(s: SemanticState): string {
  if (s.error) return s.error;
  if (!s.model?.installed || !s.serverInstalled) return t.notInstalled;
  if (!s.enabled) return t.off;
  if (s.indexing) return t.indexing(s.indexed, s.total);
  return t.indexed(s.indexed, s.total);
}

/** Settings → Search by meaning: the embedding model and the card index. */
export function SemanticSection() {
  const [state, setState] = useState<SemanticState | null>(null);

  useEffect(() => {
    let alive = true;
    void getSemanticState()
      .then((s) => alive && setState(s))
      .catch(() => undefined);
    const un = onSemanticState((s) => alive && setState(s));
    return () => {
      alive = false;
      void un.then((f) => f()).catch(() => undefined);
    };
  }, []);

  if (!state?.model) return null;
  const model = state.model;
  const dl = state.download;
  const pct = dl && dl.total ? Math.floor((dl.received / dl.total) * 100) : 0;
  const update = (p: Promise<SemanticState>) => void p.then(setState).catch(() => undefined);

  return (
    <section className={styles.section} aria-labelledby="semantic-title">
      <h2 id="semantic-title" className={styles.h2}>
        {t.settingsTitle}
      </h2>
      <p className={styles.lead}>{t.settingsLead}</p>
      <p className={styles.status} data-ready={state.ready || undefined}>
        {statusLine(state)}
      </p>

      <div className={styles.models}>
        <div className={styles.model} data-active={state.ready || undefined}>
          <div className={styles.modelHead}>
            <div className={styles.modelName}>{model.name}</div>
            <span className={styles.meta}>
              {settingsCopy.size(model.sizeBytes)} · {settingsCopy.ram(model.ramGb)}
            </span>
          </div>
          <p className={styles.modelDesc}>{model.description}</p>
          {dl ? (
            <div className={styles.progressRow}>
              <div className={styles.track}>
                <div className={styles.fill} style={{ width: `${pct}%` }} />
              </div>
              <span className={styles.meta}>{t.downloading(pct)}</span>
              <button
                type="button"
                className={styles.ghost}
                onClick={() => void cancelSemanticDownload()}
              >
                <X size={14} strokeWidth={1.75} aria-hidden="true" />
                {settingsCopy.cancel}
              </button>
            </div>
          ) : model.installed && state.serverInstalled ? (
            <div className={styles.actions}>
              <button
                type="button"
                className={styles.ghost}
                disabled={!state.enabled || state.indexing}
                onClick={() => update(reindexSemantic())}
              >
                <RefreshCw size={14} strokeWidth={1.75} aria-hidden="true" />
                {t.rebuild}
              </button>
              <button
                type="button"
                className={styles.ghost}
                onClick={() => update(removeSemanticModel())}
              >
                <Trash2 size={14} strokeWidth={1.75} aria-hidden="true" />
                {settingsCopy.remove}
              </button>
            </div>
          ) : (
            <div className={styles.actions}>
              <button
                type="button"
                className={styles.primary}
                onClick={() => void downloadSemanticModel().catch(() => undefined)}
              >
                <Download size={14} strokeWidth={1.75} aria-hidden="true" />
                {settingsCopy.download}
              </button>
            </div>
          )}
        </div>
      </div>

      <label className={styles.toggleRow}>
        <span className={styles.toggleText}>
          <span className={styles.toggleLabel}>{t.toggle}</span>
          <span className={styles.meta}>{t.toggleHint}</span>
        </span>
        <input
          type="checkbox"
          role="switch"
          className={styles.switch}
          checked={state.enabled}
          onChange={(e) => update(configureSemantic(e.target.checked))}
        />
      </label>
    </section>
  );
}
