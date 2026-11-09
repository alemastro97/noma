import { Check, Download } from "lucide-react";
import { useState } from "react";
import { settingSet, voiceStart } from "../../api/desktop";
import type { DownloadProgress } from "../../api/types";
import { onboardingCopy as t } from "../../copy";
import { useAiStore } from "../../stores/ai";
import { NomaOrb } from "../NomaOrb";
import styles from "./Onboarding.module.css";

const STEPS = 3;

function pct(dl: DownloadProgress | null): number {
  return dl && dl.total ? Math.floor((dl.received / dl.total) * 100) : 0;
}

function ModelRow({
  label,
  name,
  modelId,
  installed,
}: {
  label: string;
  name: string;
  modelId: string | undefined;
  installed: boolean;
}) {
  const state = useAiStore((s) => s.state);
  const download = useAiStore((s) => s.download);
  const dl = state?.download && state.download.modelId === modelId ? state.download : null;
  const busy = !!state?.download;
  return (
    <div className={styles.model}>
      <div className={styles.modelText}>
        <span className={styles.modelLabel}>{label}</span>
        <span className={styles.meta}>{name}</span>
      </div>
      {installed ? (
        <span className={styles.ready}>
          <Check size={14} strokeWidth={2} aria-hidden="true" />
          {t.ready}
        </span>
      ) : dl ? (
        <div className={styles.progress}>
          <div className={styles.track}>
            <div className={styles.fill} style={{ width: `${pct(dl)}%` }} />
          </div>
          <span className={styles.meta}>{pct(dl)}%</span>
        </div>
      ) : (
        <button
          type="button"
          className={styles.secondary}
          disabled={busy || !modelId}
          onClick={() => modelId && void download(modelId)}
        >
          <Download size={14} strokeWidth={1.75} aria-hidden="true" />
          {t.download}
        </button>
      )}
    </div>
  );
}

/** First-run welcome: what NOMA is, the two local models, and ⌥ Space. */
export function Onboarding({ onDone }: { onDone: () => void }) {
  const [step, setStep] = useState(0);
  const state = useAiStore((s) => s.state);

  const llm = state?.models.find((m) => m.id === state.modelId) ?? state?.models[0];
  const speech =
    state?.speechModels?.find((m) => m.id === state.speechModelId) ?? state?.speechModels?.[0];

  const finish = () => {
    void settingSet("onboarding.done", "1").catch(() => undefined);
    onDone();
  };

  return (
    <div className={styles.backdrop} role="dialog" aria-modal="true" aria-labelledby="onb-title">
      <div className={styles.card}>
        <div className={styles.orb}>
          <NomaOrb size={step === 2 ? 72 : 64} state={step === 2 ? "listening" : "idle"} />
        </div>

        {step === 0 && (
          <>
            <h1 id="onb-title" className={styles.h1}>
              {t.welcomeTitle}
            </h1>
            <p className={styles.body}>{t.welcomeBody}</p>
          </>
        )}

        {step === 1 && (
          <>
            <h1 id="onb-title" className={styles.h1}>
              {t.modelsTitle}
            </h1>
            <p className={styles.body}>{t.modelsBody}</p>
            <div className={styles.models}>
              <ModelRow
                label={t.brain}
                name={llm?.name ?? "—"}
                modelId={llm?.id}
                installed={!!llm?.installed}
              />
              <ModelRow
                label={t.ears}
                name={speech?.name ?? "—"}
                modelId={speech?.id}
                installed={!!speech?.installed}
              />
            </div>
          </>
        )}

        {step === 2 && (
          <>
            <h1 id="onb-title" className={styles.h1}>
              {t.tryTitle}
            </h1>
            <p className={styles.body}>{t.tryBody}</p>
          </>
        )}

        <footer className={styles.foot}>
          <span className={styles.meta}>{t.step(step + 1, STEPS)}</span>
          <div className={styles.buttons}>
            {step === 1 && (
              <button type="button" className={styles.ghost} onClick={() => setStep(2)}>
                {t.later}
              </button>
            )}
            {step === 2 && state?.speechReady && (
              <button type="button" className={styles.secondary} onClick={() => void voiceStart()}>
                {t.tryNow}
              </button>
            )}
            <button
              type="button"
              className={styles.primary}
              onClick={() => (step < STEPS - 1 ? setStep(step + 1) : finish())}
            >
              {step === 0 ? t.start : step === STEPS - 1 ? t.finish : t.next}
            </button>
          </div>
        </footer>
      </div>
    </div>
  );
}
