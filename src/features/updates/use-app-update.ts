import { useStore } from "zustand";

import { updateStore, type UpdatePhase, type UpdateStoreState } from "@/features/updates/update-store";

/** Current update state plus the `check`, `install` and `relaunch` actions. */
export function useAppUpdate(): UpdateStoreState {
  return useStore(updateStore);
}

/** Only the phase, for the small "update available" dot on the navigation rail. */
export function useAppUpdatePhase(): UpdatePhase {
  return useStore(updateStore, (state) => state.phase);
}
