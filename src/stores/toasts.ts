import { create } from "zustand";

export interface ToastAction {
  label: string;
  run: () => void;
}

export interface Toast {
  id: number;
  message: string;
  tone?: "default" | "error";
  action?: ToastAction;
}

interface ToastsState {
  toasts: Toast[];
  push: (toast: Omit<Toast, "id">, ms?: number) => number;
  dismiss: (id: number) => void;
}

let nextId = 1;
const timers = new Map<number, ReturnType<typeof setTimeout>>();

export const TOAST_MS = 5000;

export const useToastsStore = create<ToastsState>()((set, get) => ({
  toasts: [],
  push(toast, ms = TOAST_MS) {
    const id = nextId++;
    set((s) => ({ toasts: [...s.toasts.slice(-2), { ...toast, id }] }));
    timers.set(
      id,
      setTimeout(() => get().dismiss(id), ms),
    );
    return id;
  },
  dismiss(id) {
    const t = timers.get(id);
    if (t) clearTimeout(t);
    timers.delete(id);
    set((s) => ({ toasts: s.toasts.filter((x) => x.id !== id) }));
  },
}));

export function toastError(message: string): void {
  useToastsStore.getState().push({ message, tone: "error" });
}
