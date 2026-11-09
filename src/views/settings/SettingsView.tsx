import { Check, Download, Trash2, X } from "lucide-react";
import { useState } from "react";
import type { ModelInfo } from "../../api/types";
import { settingsCopy as t } from "../../copy";
import { NomaOrb } from "../../components/NomaOrb";
import { aiStatusLabel, useAiStore } from "../../stores/ai";
import { readTheme, setTheme, type Theme } from "../../theme/theme";
import styles from "./Settings.module.css";

function Segmented<T extends string>({
  value,
  options,
  onChange,
  label,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
  label: string;
}) {
  return (
    <div className={styles.segmented} role="radiogroup" aria-label={label}>
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          role="radio"
          aria-checked={o.value === value}
          className={styles.segment}
          onClick={() => onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

function ModelCard({ model }: { model: ModelInfo }) {
  const state = useAiStore((s) => s.state);
  const { download, cancelDownload, removeModel, configure } = useAiStore.getState();
  const dl = state?.download?.modelId === model.id ? state.download : null;
  const busy = !!state?.download;
  const inUse = state?.provider === "builtin" && state.modelId === model.id && model.installed;
  const pct = dl && dl.total ? Math.floor((dl.received / dl.total) * 100) : 0;

  return (
    <div className={styles.model} data-active={inUse || undefined}>
      <div className={styles.modelHead}>
        <div className={styles.modelName}>
          {model.name}
          {model.recommended && <span className={styles.badge}>{t.recommended}</span>}
        </div>
        <span className={styles.meta}>
          {t.size(model.sizeBytes)} · {t.ram(model.ramGb)}
        </span>
      </div>
      <p className={styles.modelDesc}>{model.description}</p>

      {dl ? (
        <div className={styles.progressRow}>
          <div className={styles.track}>
            <div className={styles.fill} style={{ width: `${pct}%` }} />
          </div>
          <span className={styles.meta}>{t.downloading(dl.stage, pct)}</span>
          <button type="button" className={styles.ghost} onClick={() => void cancelDownload()}>
            <X size={14} strokeWidth={1.75} aria-hidden="true" />
            {t.cancel}
          </button>
        </div>
      ) : model.installed ? (
        <div className={styles.actions}>
          {inUse ? (
            <span className={styles.inUse}>
              <Check size={14} strokeWidth={2} aria-hidden="true" />
              {t.inUse}
            </span>
          ) : (
            <button
              type="button"
              className={styles.primary}
              onClick={() => void configure({ provider: "builtin", modelId: model.id })}
            >
              {t.use}
            </button>
          )}
          <button type="button" className={styles.ghost} onClick={() => void removeModel(model.id)}>
            <Trash2 size={14} strokeWidth={1.75} aria-hidden="true" />
            {t.remove}
          </button>
        </div>
      ) : (
        <div className={styles.actions}>
          <button
            type="button"
            className={styles.primary}
            disabled={busy}
            onClick={() => void download(model.id)}
          >
            <Download size={14} strokeWidth={1.75} aria-hidden="true" />
            {t.download}
          </button>
        </div>
      )}
    </div>
  );
}

export function SettingsView() {
  const state = useAiStore((s) => s.state);
  const orb = useAiStore((s) => s.orb);
  const configure = useAiStore((s) => s.configure);
  const [theme, setThemeState] = useState<Theme>(readTheme);

  return (
    <div className={styles.scroll}>
      <div className={styles.page}>
        <section className={styles.section} aria-labelledby="ai-title">
          <div className={styles.sectionHead}>
            <NomaOrb size={28} state={orb} label={aiStatusLabel(state, orb)} />
            <div>
              <h2 id="ai-title" className={styles.h2}>
                {t.aiTitle}
              </h2>
              <p className={styles.lead}>{t.aiLead}</p>
            </div>
          </div>

          {state && (
            <>
              <div className={styles.row}>
                <span className={styles.label}>{t.provider}</span>
                <Segmented
                  label={t.provider}
                  value={state.provider}
                  options={[
                    { value: "builtin", label: t.builtin },
                    { value: "ollama", label: t.ollama },
                  ]}
                  onChange={(provider) => void configure({ provider })}
                />
              </div>

              <p className={styles.status} data-ready={state.ready || undefined}>
                {state.message && state.phase === "error"
                  ? state.message
                  : state.ready
                    ? t.ready
                    : t.notReady}
                {state.pending > 0 && ` ${t.pending(state.pending)}`}
              </p>

              {state.provider === "builtin" ? (
                <div className={styles.models}>
                  {state.models.map((m) => (
                    <ModelCard key={m.id} model={m} />
                  ))}
                </div>
              ) : (
                <div className={styles.ollama}>
                  <p className={styles.lead}>{t.ollamaLead}</p>
                  {state.ollamaAvailable ? (
                    <select
                      className={styles.select}
                      value={state.ollamaModel ?? ""}
                      onChange={(e) => void configure({ ollamaModel: e.target.value })}
                    >
                      <option value="" disabled>
                        {t.ollamaPick}
                      </option>
                      {state.ollamaModels.map((m) => (
                        <option key={m} value={m}>
                          {m}
                        </option>
                      ))}
                    </select>
                  ) : (
                    <p className={styles.status}>{t.ollamaMissing}</p>
                  )}
                </div>
              )}
            </>
          )}
        </section>

        <section className={styles.section} aria-labelledby="appearance-title">
          <h2 id="appearance-title" className={styles.h2}>
            {t.appearance}
          </h2>
          <Segmented<Theme>
            label={t.appearance}
            value={theme}
            options={[
              { value: "dark", label: t.theme.dark },
              { value: "light", label: t.theme.light },
              { value: "system", label: t.theme.system },
            ]}
            onChange={(v) => {
              setTheme(v);
              setThemeState(v);
            }}
          />
        </section>

        <section className={styles.section} aria-labelledby="about-title">
          <h2 id="about-title" className={styles.h2}>
            {t.about}
          </h2>
          <p className={styles.lead}>{t.version}</p>
        </section>
      </div>
    </div>
  );
}
