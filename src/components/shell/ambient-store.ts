import { create } from "zustand";

import type { GameSummary } from "@/lib/bindings/GameSummary";

interface AmbientState {
  /** The game whose artwork tints the page background; `null` leaves the plain canvas. */
  game: GameSummary | null;
  setGame: (game: GameSummary | null) => void;
}

/** Lets a page ask the shell for an ambient backdrop (the dashboard follows its hero carousel). */
export const useAmbient = create<AmbientState>((set) => ({
  game: null,
  setGame: (game) => set({ game }),
}));
