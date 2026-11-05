import { create } from "zustand";

export type ToastTone = "success" | "neutral" | "warning" | "danger";

export interface Toast {
  id: number;
  tone: ToastTone;
  title: string;
  body?: string | undefined;
}

interface ToastState {
  toasts: Toast[];
  push: (toast: Omit<Toast, "id">) => void;
  dismiss: (id: number) => void;
}

const AUTO_DISMISS_MS = 5000;
let nextId = 1;

/** Notification toasts: every operation the user starts ends with a visible result. */
export const useToasts = create<ToastState>((set, get) => ({
  toasts: [],
  push: (toast) => {
    const id = nextId++;
    set({ toasts: [...get().toasts.slice(-2), { ...toast, id }] });
    window.setTimeout(() => get().dismiss(id), AUTO_DISMISS_MS);
  },
  dismiss: (id) => set({ toasts: get().toasts.filter((toast) => toast.id !== id) }),
}));

/** Shortcut usable outside React components. */
export const notify = (toast: Omit<Toast, "id">) => useToasts.getState().push(toast);
