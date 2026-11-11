import { useState } from "react";
import { voiceCopy as t } from "../../copy";
import { useSetting } from "../../hooks/useSetting";
import styles from "./Settings.module.css";

function Switch({
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

/** Words Whisper should expect; saved on blur so typing doesn't write every key. */
function Vocabulary() {
  const [saved, save] = useSetting("voice.vocabulary", "");
  const [draft, setDraft] = useState<string | null>(null);
  const value = draft ?? saved ?? "";
  return (
    <label className={styles.toggleRow}>
      <span className={styles.toggleText}>
        <span className={styles.toggleLabel}>{t.vocabulary}</span>
        <span className={styles.meta}>{t.vocabularyHint}</span>
      </span>
      <input
        type="text"
        className={styles.select}
        value={value}
        placeholder={t.vocabularyPlaceholder}
        disabled={saved === null}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={() => {
          if (draft !== null && draft !== saved) save(draft.trim());
          setDraft(null);
        }}
      />
    </label>
  );
}

/** Settings → voice commands, sounds, spoken answers, kept audio, vocabulary. */
export function VoiceExtrasSection() {
  const [sound, setSound] = useSetting("voice.sound", "1");
  const [speak, setSpeak] = useSetting("voice.speak", "1");
  const [keep, setKeep] = useSetting("voice.keepAudio", "0");
  const on = (v: string | null) => (v === null ? null : v === "1");
  return (
    <section className={styles.section} aria-labelledby="voice-extras-title">
      <h2 id="voice-extras-title" className={styles.h2}>
        {t.extrasTitle}
      </h2>
      <p className={styles.lead}>{t.extrasLead}</p>
      <Switch
        label={t.sound}
        hint={t.soundHint}
        value={on(sound)}
        onChange={(v) => setSound(v ? "1" : "0")}
      />
      <Switch
        label={t.speak}
        hint={t.speakHint}
        value={on(speak)}
        onChange={(v) => setSpeak(v ? "1" : "0")}
      />
      <Switch
        label={t.keepAudio}
        hint={t.keepAudioHint}
        value={on(keep)}
        onChange={(v) => setKeep(v ? "1" : "0")}
      />
      <Vocabulary />
    </section>
  );
}
