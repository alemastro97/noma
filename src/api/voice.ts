import { invoke } from "@tauri-apps/api/core";

/** Original audio kept for a voice capture (Settings → Voice → Keep the recording). */
export interface VoiceRecording {
  id: string;
  captureId: string;
  durationMs: number;
  createdAt: string;
}

export const voiceRecordingForItem = (itemId: string) =>
  invoke<VoiceRecording | null>("voice_recording_for_item", { itemId });

/** The recording as WAV bytes. */
export const voiceRecordingAudio = (id: string) =>
  invoke<ArrayBuffer>("voice_recording_audio", { id });

/** Puts a card back the way it was before a voice command. */
export const voiceCommandUndo = (toastId: string) =>
  invoke<void>("voice_command_undo", { toastId });

/** Toasts confirming a voice command have ids `voice:done:<item>` / `voice:moved:<item>`. */
export function voiceToastKind(id: string): "done" | "moved" | null {
  if (id.startsWith("voice:done:")) return "done";
  if (id.startsWith("voice:moved:")) return "moved";
  return null;
}
