import { create } from "zustand";
import * as api from "../api/ai";
import type { AiConfigPatch, AiState, CaptureContext, CaptureEvent, Item } from "../api/types";
import type { OrbState } from "../components/NomaOrb";
import { SUCCESS_FLASH_MS } from "../components/NomaOrb";
import { useItemsStore } from "./items";
import { toastError, useToastsStore } from "./toasts";

interface AiStore {
  state: AiState | null;
  /** Orb shown in the header: follows the AI phase, flashes on capture results. */
  orb: OrbState;
  load: () => Promise<void>;
  subscribe: () => Promise<() => void>;
  configure: (patch: AiConfigPatch) => Promise<void>;
  download: (modelId: string) => Promise<void>;
  cancelDownload: () => Promise<void>;
  removeModel: (modelId: string) => Promise<void>;
  /** Sends free text to AI capture; the placeholder card appears immediately. */
  capture: (text: string, context: CaptureContext) => Promise<Item | null>;
  retry: (itemId: string) => Promise<void>;
}

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function orbFor(state: AiState | null): OrbState {
  if (!state) return "idle";
  if (state.phase === "loading" || state.phase === "processing") return "processing";
  if (state.phase === "error") return "error";
  return "idle";
}

let flashTimer: ReturnType<typeof setTimeout> | null = null;

export const useAiStore = create<AiStore>()((set, get) => {
  const flash = (orb: OrbState, ms: number) => {
    if (flashTimer) clearTimeout(flashTimer);
    set({ orb });
    flashTimer = setTimeout(() => {
      flashTimer = null;
      set({ orb: orbFor(get().state) });
    }, ms);
  };

  const onCapture = (event: CaptureEvent) => {
    if (event.status === "done") {
      flash("success", SUCCESS_FLASH_MS);
      const first = event.itemIds[0];
      if (first) useItemsStore.setState({ freshId: first });
    } else {
      flash("error", 2500);
      const itemId = event.itemIds[0];
      useToastsStore.getState().push({
        message: "Couldn't read that capture. It's kept as a note.",
        tone: "error",
        action: itemId ? { label: "Retry", run: () => void get().retry(itemId) } : undefined,
      });
    }
  };

  return {
    state: null,
    orb: "idle",

    async load() {
      try {
        const state = await api.getAiState();
        set({ state, orb: flashTimer ? get().orb : orbFor(state) });
      } catch {
        // Backend without AI (tests, older builds): stay idle.
      }
    },

    async subscribe() {
      const offState = await api.onAiState((state) =>
        set({ state, orb: flashTimer ? get().orb : orbFor(state) }),
      );
      const offCapture = await api.onCapture(onCapture);
      return () => {
        offState();
        offCapture();
      };
    },

    async configure(patch) {
      try {
        set({ state: await api.configureAi(patch) });
      } catch (err) {
        toastError(message(err));
      }
    },

    async download(modelId) {
      try {
        await api.downloadModel(modelId);
      } catch (err) {
        toastError(message(err));
      }
    },

    async cancelDownload() {
      await api.cancelDownload();
    },

    async removeModel(modelId) {
      try {
        set({ state: await api.removeModel(modelId) });
      } catch (err) {
        toastError(message(err));
      }
    },

    async capture(text, context) {
      try {
        const { item } = await api.captureText(text, context);
        useItemsStore.setState((s) => ({
          items: { ...s.items, [item.id]: item },
          order: [item.id, ...s.order.filter((id) => id !== item.id)],
          freshId: item.id,
        }));
        return item;
      } catch (err) {
        toastError(message(err));
        return null;
      }
    },

    async retry(itemId) {
      try {
        await api.retryCapture(itemId);
      } catch (err) {
        toastError(message(err));
      }
    },
  };
});

/** Short status for the header chip and the orb label. */
export function aiStatusLabel(state: AiState | null, orb: OrbState): string {
  if (orb === "success") return "Captured";
  if (!state) return "Idle";
  if (state.download) {
    const pct = state.download.total
      ? Math.floor((state.download.received / state.download.total) * 100)
      : 0;
    return `Downloading ${pct}%`;
  }
  if (state.phase === "loading") return "Waking up";
  if (state.phase === "processing") return "Thinking";
  if (state.phase === "error") return "Needs attention";
  if (!state.ready) return state.pending > 0 ? `${state.pending} waiting` : "Set up AI";
  return "Idle";
}
