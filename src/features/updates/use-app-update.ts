import { useStore } from "zustand";

import { updateStore, type UpdatePhase, type UpdateStoreState } from "@/features/updates/update-store";

/** Current update state plus the `check`, `install` and `relaunch` actions. */
export function useAppUpdate(): UpdateStoreState {
  return useStore(updateStore);
}

/** An update is waiting to be downloaded, is downloading, or is ready to install: the user has something to do. */
export function isUpdatePending(phase: UpdatePhase): boolean {
  return phase === "available" || phase === "downloading" || phase === "ready";
}

/** Only the phase, for the small "update available" dot on the navigation rail. */
export function useAppUpdatePhase(): UpdatePhase {
  return useStore(updateStore, (state) => state.phase);
}
