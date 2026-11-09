import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AiConfigPatch, AiState, CaptureContext, CaptureEvent, CaptureResult } from "./types";

export const AI_STATE = "ai:state";
export const AI_CAPTURE = "ai:capture";

export function getAiState(): Promise<AiState> {
  return invoke<AiState>("ai_state");
}

/** Re-checks Ollama and wakes the capture worker. */
export function refreshAi(): Promise<AiState> {
  return invoke<AiState>("ai_refresh");
}

export function configureAi(patch: AiConfigPatch): Promise<AiState> {
  return invoke<AiState>("ai_configure", { patch });
}

export function downloadModel(modelId: string): Promise<void> {
  return invoke<void>("ai_download", { modelId });
}

export function cancelDownload(): Promise<void> {
  return invoke<void>("ai_cancel_download");
}

export function removeModel(modelId: string): Promise<AiState> {
  return invoke<AiState>("ai_remove_model", { modelId });
}

export function captureText(text: string, context: CaptureContext): Promise<CaptureResult> {
  return invoke<CaptureResult>("capture_text", { text, context });
}

export function retryCapture(itemId: string): Promise<void> {
  return invoke<void>("capture_retry", { itemId });
}

export function onAiState(cb: (state: AiState) => void): Promise<UnlistenFn> {
  return listen<AiState>(AI_STATE, (e) => cb(e.payload));
}

export function onCapture(cb: (event: CaptureEvent) => void): Promise<UnlistenFn> {
  return listen<CaptureEvent>(AI_CAPTURE, (e) => cb(e.payload));
}
