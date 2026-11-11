import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Item } from "./types";

export const SEMANTIC_STATE = "semantic:state";

export interface EmbeddingInfo {
  id: string;
  name: string;
  description: string;
  sizeBytes: number;
  ramGb: number;
  installed: boolean;
}

export interface SemanticState {
  enabled: boolean;
  /** Enabled and installed: the index is kept up to date. */
  ready: boolean;
  model: EmbeddingInfo | null;
  serverInstalled: boolean;
  download: { received: number; total: number } | null;
  indexing: boolean;
  indexed: number;
  total: number;
  running: boolean;
  error: string | null;
}

export interface SemanticHit {
  item: Item;
  score: number;
}

export interface AskTurn {
  question: string;
  answer: string;
}

export interface AskAnswer {
  /** Empty when no card relates to the question. */
  answer: string;
  /** Cards given to the model; `[1]` in the answer is `cards[0]`. */
  cards: Item[];
  /** 1-based card numbers the answer cites. */
  cited: number[];
}

export function getSemanticState(): Promise<SemanticState> {
  return invoke<SemanticState>("semantic_state");
}

export function configureSemantic(enabled: boolean): Promise<SemanticState> {
  return invoke<SemanticState>("semantic_configure", { enabled });
}

export function downloadSemanticModel(): Promise<void> {
  return invoke<void>("semantic_download");
}

export function cancelSemanticDownload(): Promise<void> {
  return invoke<void>("semantic_cancel_download");
}

export function removeSemanticModel(): Promise<SemanticState> {
  return invoke<SemanticState>("semantic_remove");
}

export function reindexSemantic(): Promise<SemanticState> {
  return invoke<SemanticState>("semantic_reindex");
}

export function semanticSearch(query: string, limit = 8): Promise<SemanticHit[]> {
  return invoke<SemanticHit[]>("semantic_search", { query, limit });
}

export function askNoma(question: string, history: AskTurn[]): Promise<AskAnswer> {
  return invoke<AskAnswer>("ask_noma", { question, history });
}

export function onSemanticState(cb: (state: SemanticState) => void): Promise<UnlistenFn> {
  return listen<SemanticState>(SEMANTIC_STATE, (e) => cb(e.payload));
}
