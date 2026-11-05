import { create } from "zustand";

import type { FolderScanReport } from "@/lib/bindings/FolderScanReport";

/** Which step of the "Add a game" flow is on screen. */
export type AddGameStage =
  { kind: "closed" } | { kind: "choose" } | { kind: "scanned"; folder: string; report: FolderScanReport };

interface AddGameState {
  stage: AddGameStage;
  openChooser: () => void;
  showScan: (folder: string, report: FolderScanReport) => void;
  close: () => void;
}

export const useAddGameStore = create<AddGameState>((set) => ({
  stage: { kind: "closed" },
  openChooser: () => set({ stage: { kind: "choose" } }),
  showScan: (folder, report) => set({ stage: { kind: "scanned", folder, report } }),
  close: () => set({ stage: { kind: "closed" } }),
}));
