import { create } from "zustand";

export type ConsoleToastTone = "neutral" | "success" | "warning" | "danger";

export interface ConsoleToast {
  id: number;
  tone: ConsoleToastTone;
  title: string;
  body?: string | undefined;
}

interface ToastStore {
  toasts: readonly ConsoleToast[];
  push: (toast: Omit<ConsoleToast, "id">) => void;
  dismiss: (id: number) => void;
}

const LIFETIME_MS = 4200;
const MAX_VISIBLE = 3;
let nextId = 1;

/** Notifications drawn inside console mode, large enough to read from the sofa. */
export const useConsoleToasts = create<ToastStore>((set, get) => ({
  toasts: [],
  push: (toast) => {
    const id = nextId++;
    set((state) => ({ toasts: [...state.toasts, { ...toast, id }].slice(-MAX_VISIBLE) }));
    window.setTimeout(() => get().dismiss(id), LIFETIME_MS);
  },
  dismiss: (id) => set((state) => ({ toasts: state.toasts.filter((toast) => toast.id !== id) })),
}));

export const consoleToast = (toast: Omit<ConsoleToast, "id">) => useConsoleToasts.getState().push(toast);
