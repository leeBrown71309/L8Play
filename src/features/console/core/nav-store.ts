import { create } from "zustand";

import { INITIAL_NAV, navReducer, type NavAction, type NavState } from "@/features/console/core/navigation";

interface NavStore extends NavState {
  dispatch: (action: NavAction) => void;
  /** Back to the library with nothing open (used when console mode is entered again). */
  reset: () => void;
}

/** The reducer behind a small store, so any console component can read and drive the navigation. */
export const useConsoleNav = create<NavStore>((set, get) => ({
  ...INITIAL_NAV,
  dispatch: (action) => {
    const { section, gameId, modals, focus } = get();
    const next = navReducer({ section, gameId, modals, focus }, action);
    if (next.section !== section || next.gameId !== gameId || next.modals !== modals || next.focus !== focus) {
      set({ section: next.section, gameId: next.gameId, modals: next.modals, focus: next.focus });
    }
  },
  reset: () => set({ ...INITIAL_NAV }),
}));

/** Focused index of a list, and a setter bound to its zone and size. */
export function useZoneFocus(zone: string, count: number) {
  const index = useConsoleNav((state) => Math.min(Math.max(0, count - 1), state.focus[zone] ?? 0));
  const dispatch = useConsoleNav((state) => state.dispatch);
  return {
    index,
    move: (delta: number) => dispatch({ type: "moveFocus", zone, delta, count }),
    set: (next: number) => dispatch({ type: "setFocus", zone, index: next, count }),
  };
}
