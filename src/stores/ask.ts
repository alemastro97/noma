import { create } from "zustand";
import { askNoma, type AskAnswer, type AskTurn } from "../api/semantic";

/** Earlier turns sent with a question (the backend keeps the last 3). */
export const HISTORY_TURNS = 3;
/** Turns kept on screen. */
const MAX_TURNS = 8;

export interface Turn {
  id: number;
  question: string;
  pending: boolean;
  result?: AskAnswer;
  error?: string;
}

interface AskStore {
  open: boolean;
  turns: Turn[];
  /** Text to put in the input when the panel opens (from the palette). */
  draft: string;
  openAsk: (draft?: string) => void;
  close: () => void;
  clear: () => void;
  ask: (question: string) => Promise<void>;
}

let seq = 0;

/** Answered turns as context for the next question. */
export function historyOf(turns: Turn[]): AskTurn[] {
  return turns
    .filter((t) => !t.pending && t.result?.answer)
    .slice(-HISTORY_TURNS)
    .map((t) => ({ question: t.question, answer: t.result!.answer }));
}

export const useAskStore = create<AskStore>()((set, get) => ({
  open: false,
  turns: [],
  draft: "",
  openAsk: (draft = "") => set({ open: true, draft }),
  close: () => set({ open: false, draft: "" }),
  clear: () => set({ turns: [] }),
  async ask(question) {
    const q = question.trim();
    if (!q || get().turns.some((t) => t.pending)) return;
    const id = ++seq;
    const history = historyOf(get().turns);
    set((s) => ({
      draft: "",
      turns: [...s.turns, { id, question: q, pending: true }].slice(-MAX_TURNS),
    }));
    const patch = (p: Partial<Turn>) =>
      set((s) => ({ turns: s.turns.map((t) => (t.id === id ? { ...t, ...p } : t)) }));
    try {
      const result = await askNoma(q, history);
      patch({ pending: false, result });
    } catch (err) {
      patch({ pending: false, error: err instanceof Error ? err.message : String(err) });
    }
  },
}));
