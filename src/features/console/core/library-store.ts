import { create } from "zustand";

import type { ConsoleFilter, Letter } from "@/features/console/core/console-filter";

interface ConsoleLibraryState {
  filter: ConsoleFilter;
  letter: Letter | null;
  /** Index of the selected cover in the filtered list. */
  selected: number;
  /** Id of the selected game, read by the backdrop (kept next to the index so both change together). */
  selectedGameId: number | null;
  setFilter: (filter: ConsoleFilter) => void;
  setLetter: (letter: Letter | null) => void;
  select: (index: number, gameId: number | null) => void;
}

/** What the coverflow shows. A new filter or letter goes back to the first cover. */
export const useConsoleLibrary = create<ConsoleLibraryState>((set) => ({
  filter: "all",
  letter: null,
  selected: 0,
  selectedGameId: null,
  setFilter: (filter) => set({ filter, selected: 0 }),
  setLetter: (letter) => set({ letter, selected: 0 }),
  select: (selected, selectedGameId) => set({ selected, selectedGameId }),
}));
