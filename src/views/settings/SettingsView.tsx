import { Check, Download, Trash2, X } from "lucide-react";
import { useEffect, useState } from "react";
import type { ModelInfo, SpeechInfo } from "../../api/types";
import type { BackupInfo } from "../../api/types";
import {
  autostartGet,
  autostartSet,
  backupInfo,
  backupNow,
  backupReveal,
  dataExport,
} from "../../api/desktop";
import { ShortcutRecorder } from "../../components/ShortcutRecorder/ShortcutRecorder";
import { useSetting } from "../../hooks/useSetting";
import { useShortcuts } from "../../hooks/useShortcuts";
import { formatAccelerator, type Action } from "../../lib/shortcuts";
import { settingsCopy as t } from "../../copy";
import { NomaOrb } from "../../components/NomaOrb";
import { aiStatusLabel, useAiStore } from "../../stores/ai";
import { readTheme, setTheme, type Theme } from "../../theme/theme";
import styles from "./Settings.module.css";
import { SemanticSection } from "./SemanticSection";
import { TimeSettings } from "./TimeSettings";
import { VoiceExtrasSection } from "./VoiceExtrasSection";
import { DesktopPresenceSettings } from "./DesktopPresenceSettings";

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

function Toggle({
  label,
  hint,
  value,
  onChange,
}: {
  label: string;
  hint: string;
  value: boolean | null;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className={styles.toggleRow}>
      <span className={styles.toggleText}>
        <span className={styles.toggleLabel}>{label}</span>
        <span className={styles.meta}>{hint}</span>
      </span>
      <input
        type="checkbox"
        role="switch"
        className={styles.switch}
        checked={!!value}
        disabled={value === null}
        onChange={(e) => onChange(e.target.checked)}
      />
    </label>
  );
}

function SpeechCard({ model }: { model: SpeechInfo }) {
  const state = useAiStore((s) => s.state);
  const { download, cancelDownload, removeModel, configure } = useAiStore.getState();
  const dl = state?.download?.modelId === model.id ? state.download : null;
  const busy = !!state?.download;
  const inUse = state?.speechModelId === model.id && model.installed;
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
              onClick={() => void configure({ speechModelId: model.id })}
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

function VoiceSection() {
  const state = useAiStore((s) => s.state);
  const [autoStop, setAutoStop] = useSetting("voice.autoStop", "0");
  const [language, setLanguage] = useSetting("voice.language", "it");
  const shortcuts = useShortcuts();
  const voiceKeys = formatAccelerator(shortcuts.voice);
  const [display, setDisplay] = useSetting("voice.display", "orb");
  if (!state) return null;
  return (
    <section className={styles.section} aria-labelledby="voice-title">
      <h2 id="voice-title" className={styles.h2}>
        {t.voiceTitle}
      </h2>
      <p className={styles.lead}>{t.voiceLead(voiceKeys)}</p>
      <p className={styles.status} data-ready={state.speechReady || undefined}>
        {state.speechReady ? t.voiceReady(voiceKeys) : t.voiceNotReady}
      </p>
      <div className={styles.models}>
        {(state.speechModels ?? []).map((m) => (
          <SpeechCard key={m.id} model={m} />
        ))}
      </div>
      <div className={styles.row}>
        <span className={styles.toggleText}>
          <span className={styles.toggleLabel}>{t.display}</span>
          <span className={styles.meta}>{t.displayHint}</span>
        </span>
        <Segmented
          label={t.display}
          value={display ?? "orb"}
          options={[
            { value: "orb", label: t.displays.orb },
            { value: "overlay", label: t.displays.overlay },
          ]}
          onChange={setDisplay}
        />
      </div>
      <div className={styles.row}>
        <span className={styles.toggleText}>
          <span className={styles.toggleLabel}>{t.language}</span>
          <span className={styles.meta}>{t.languageHint}</span>
        </span>
        <Segmented
          label={t.language}
          value={language ?? "it"}
          options={[
            { value: "it", label: t.languages.it },
            { value: "en", label: t.languages.en },
            { value: "auto", label: t.languages.auto },
          ]}
          onChange={setLanguage}
        />
      </div>
      <Toggle
        label={t.autoStop}
        hint={t.autoStopHint}
        value={autoStop === null ? null : autoStop !== "0"}
        onChange={(v) => setAutoStop(v ? "1" : "0")}
      />
    </section>
  );
}

function ShortcutBinding({ action, label }: { action: Action; label: string }) {
  const shortcuts = useShortcuts();
  const [override, setOverride] = useState<string | null>(null);
  const value = override ?? shortcuts[action];
  return (
    <div className={styles.shortcut}>
      <span>{label}</span>
      <ShortcutRecorder action={action} value={value} onChange={setOverride} />
    </div>
  );
}

function DesktopSection() {
  const [orb, setOrb] = useSetting("orb.enabled", "1");
  const [paused, setPaused] = useSetting("reminders.paused", "0");
  const [native, setNative] = useSetting("reminders.native", "1");
  const [filed, setFiled] = useSetting("capture.toast", "1");
  const [brief, setBrief] = useSetting("reminders.brief", "1");
  const [login, setLogin] = useState<boolean | null>(null);
  useEffect(() => {
    void autostartGet()
      .then(setLogin)
      .catch(() => setLogin(false));
  }, []);
  const bool = (v: string | null, on: string) => (v === null ? null : v === on);
  return (
    <section className={styles.section} aria-labelledby="desktop-title">
      <h2 id="desktop-title" className={styles.h2}>
        {t.desktopTitle}
      </h2>
      <Toggle
        label={t.openAtLogin}
        hint={t.openAtLoginHint}
        value={login}
        onChange={(v) => {
          setLogin(v);
          void autostartSet(v)
            .then(setLogin)
            .catch(() => setLogin(!v));
        }}
      />
      <Toggle
        label={t.orbToggle}
        hint={t.orbHint}
        value={bool(orb, "1")}
        onChange={(v) => setOrb(v ? "1" : "0")}
      />
      <Toggle
        label={t.filedToast}
        hint={t.filedToastHint}
        value={bool(filed, "1")}
        onChange={(v) => setFiled(v ? "1" : "0")}
      />
      <Toggle
        label={t.brief}
        hint={t.briefHint}
        value={bool(brief, "1")}
        onChange={(v) => setBrief(v ? "1" : "0")}
      />
      <Toggle
        label={t.nativeNotifications}
        hint={t.nativeHint}
        value={bool(native, "1")}
        onChange={(v) => setNative(v ? "1" : "0")}
      />
      <Toggle
        label={t.remindersPaused}
        hint={t.remindersPausedHint}
        value={bool(paused, "1")}
        onChange={(v) => setPaused(v ? "1" : "0")}
      />
      <div className={styles.shortcuts}>
        <span className={styles.label}>{t.shortcuts}</span>
        <p className={styles.meta}>{t.shortcutHint}</p>
        <ShortcutBinding action="voice" label={t.shortcutVoice} />
        <ShortcutBinding action="capture" label={t.shortcutCapture} />
      </div>
      <DesktopPresenceSettings />
    </section>
  );
}

function DataSection() {
  const [info, setInfo] = useState<BackupInfo | null>(null);
  const [exported, setExported] = useState(false);
  useEffect(() => {
    void backupInfo()
      .then(setInfo)
      .catch(() => undefined);
  }, []);
  const last = info?.lastAt
    ? new Date(info.lastAt).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" })
    : null;
  return (
    <section className={styles.section} aria-labelledby="data-title">
      <h2 id="data-title" className={styles.h2}>
        {t.dataTitle}
      </h2>
      <p className={styles.lead}>{t.dataLead}</p>
      <p className={styles.status}>{exported ? t.exported : t.lastBackup(last)}</p>
      <div className={styles.dataActions}>
        <button
          type="button"
          className={styles.ghost}
          onClick={() =>
            void backupNow()
              .then(setInfo)
              .catch(() => undefined)
          }
        >
          {t.backupNow}
        </button>
        <button type="button" className={styles.ghost} onClick={() => void backupReveal()}>
          {t.showBackups}
        </button>
        <button
          type="button"
          className={styles.ghost}
          onClick={() =>
            void dataExport()
              .then(() => setExported(true))
              .catch(() => undefined)
          }
        >
          {t.exportData}
        </button>
      </div>
    </section>
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

        <VoiceSection />
        <VoiceExtrasSection />
        <SemanticSection />
        <DesktopSection />
        <TimeSettings />
        <DataSection />

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
