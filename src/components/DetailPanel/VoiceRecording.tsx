import { Play, Square } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { voiceRecordingAudio, voiceRecordingForItem, type VoiceRecording } from "../../api/voice";
import { voiceCopy } from "../../copy";
import styles from "./DetailPanel.module.css";
import own from "./VoiceRecording.module.css";

/** Play button for the original audio of a voice capture, when it was kept. */
export function VoiceRecordingField({ itemId }: { itemId: string }) {
  const [rec, setRec] = useState<VoiceRecording | null>(null);
  const [playing, setPlaying] = useState(false);
  const audio = useRef<HTMLAudioElement | null>(null);
  const url = useRef<string | null>(null);

  useEffect(() => {
    let alive = true;
    void voiceRecordingForItem(itemId)
      .then((r) => alive && setRec(r))
      .catch(() => undefined);
    return () => {
      alive = false;
      audio.current?.pause();
      if (url.current) URL.revokeObjectURL(url.current);
      audio.current = null;
      url.current = null;
    };
  }, [itemId]);

  if (!rec) return null;

  const toggle = async () => {
    if (playing) {
      audio.current?.pause();
      setPlaying(false);
      return;
    }
    if (!audio.current) {
      const bytes = await voiceRecordingAudio(rec.id).catch(() => null);
      if (!bytes || bytes.byteLength === 0) return;
      url.current = URL.createObjectURL(new Blob([bytes], { type: "audio/wav" }));
      audio.current = new Audio(url.current);
      audio.current.onended = () => setPlaying(false);
    }
    audio.current.currentTime = 0;
    void audio.current.play().then(
      () => setPlaying(true),
      () => setPlaying(false),
    );
  };

  return (
    <div className={styles.field}>
      <span className={styles.label}>{voiceCopy.recording}</span>
      <button
        type="button"
        className={own.play}
        aria-label={playing ? voiceCopy.stop : voiceCopy.play}
        onClick={() => void toggle()}
      >
        {playing ? (
          <Square size={13} strokeWidth={1.75} aria-hidden="true" />
        ) : (
          <Play size={13} strokeWidth={1.75} aria-hidden="true" />
        )}
        <span>{voiceCopy.duration(rec.durationMs)}</span>
      </button>
    </div>
  );
}
