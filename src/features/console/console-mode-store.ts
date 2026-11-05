import { create } from "zustand";

import { consoleSound } from "@/features/console/sound";

interface ConsoleModeState {
  /** True while the console interface covers the desktop. Never persisted: the app always starts on the desktop. */
  active: boolean;
  /** Call from the click or key press that opens console mode: the browser only allows audio after a gesture. */
  enter: () => void;
  exit: () => void;
  toggle: () => void;
}

export const useConsoleMode = create<ConsoleModeState>((set, get) => ({
  active: false,
  enter: () => {
    if (get().active) return;
    set({ active: true });
    void consoleSound.unlock().then(() => consoleSound.play("boot"));
  },
  exit: () => {
    if (!get().active) return;
    set({ active: false });
  },
  toggle: () => (get().active ? get().exit() : get().enter()),
}));
