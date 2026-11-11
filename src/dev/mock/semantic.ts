/**
 * Mock backend cases for semantic search and Ask NOMA (browser dev mode).
 * "Meaning" is faked with any-word matching over the mock cards.
 */
import { invoke } from "@tauri-apps/api/core";
import type { AskAnswer, SemanticHit, SemanticState } from "../../api/semantic";
import type { Item } from "../../api/types";

const state: SemanticState = {
  enabled: true,
  ready: true,
  model: {
    id: "embeddinggemma-300m-q8",
    name: "EmbeddingGemma 300M",
    description: "Finds cards by meaning, in Italian and English.",
    sizeBytes: 333_590_944,
    ramGb: 0.5,
    installed: true,
  },
  serverInstalled: true,
  download: null,
  indexing: false,
  indexed: 0,
  total: 0,
  running: false,
  error: null,
};

async function related(text: string, limit: number): Promise<Item[]> {
  const words = text
    .toLowerCase()
    .split(/[^\p{L}\p{N}]+/u)
    .filter((w) => w.length >= 4);
  const seen = new Map<string, Item>();
  for (const w of words) {
    for (const item of await invoke<Item[]>("items_search", { query: w, limit })) {
      if (!seen.has(item.id)) seen.set(item.id, item);
    }
  }
  return [...seen.values()].slice(0, limit);
}

async function snapshot(): Promise<SemanticState> {
  const all = await invoke<Item[]>("items_search", { query: "", limit: 10_000 });
  const ready = state.enabled && !!state.model?.installed;
  return { ...state, ready, indexed: ready ? all.length : 0, total: all.length };
}

/** Returns undefined for commands that are not semantic ones. */
export function handleSemantic(cmd: string, a: Record<string, unknown>): unknown {
  switch (cmd) {
    case "semantic_state":
    case "semantic_reindex":
      return snapshot();
    case "semantic_configure":
      state.enabled = Boolean(a.enabled);
      return snapshot();
    case "semantic_download":
      if (state.model) state.model.installed = true;
      return null;
    case "semantic_cancel_download":
      return null;
    case "semantic_remove":
      if (state.model) state.model.installed = false;
      return snapshot();
    case "semantic_search":
      return related(String(a.query ?? ""), Number(a.limit ?? 8)).then((items) =>
        items.map((item): SemanticHit => ({ item, score: 0.5 })),
      );
    case "ask_noma":
      return new Promise<AskAnswer>((resolve) => {
        void related(String(a.question ?? ""), 6).then((cards) =>
          setTimeout(
            () =>
              resolve({
                answer: cards[0]
                  ? `From your cards: ${cards[0].title} [1].` +
                    (cards[1] ? ` See also ${cards[1].title} [2].` : "")
                  : "",
                cards,
                cited: cards.slice(0, 2).map((_, i) => i + 1),
              }),
            700,
          ),
        );
      });
    default:
      return undefined;
  }
}
