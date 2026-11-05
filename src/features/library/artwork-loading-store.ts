import { create } from "zustand";

/** A fetch that never reports back must not leave a card shimmering forever. */
export const ARTWORK_LOADING_TIMEOUT_MS = 20_000;
/** An "artwork fetched" event can beat the command response; ignore marks that arrive just after it. */
const LATE_MARK_WINDOW_MS = 10_000;

interface ArtworkLoadingState {
  /** Games whose artwork is being looked up right now. */
  pending: ReadonlySet<number>;
  markPending: (gameIds: readonly number[]) => void;
  finish: (gameId: number) => void;
}

const timers = new Map<number, ReturnType<typeof setTimeout>>();
const finishedAt = new Map<number, number>();

/**
 * Games just added by the user whose artwork is still being fetched in the background.
 * Cards, rows, tiles and rail icons show the design system's shimmer for them.
 */
export const useArtworkLoading = create<ArtworkLoadingState>((set, get) => ({
  pending: new Set<number>(),
  markPending: (gameIds) => {
    const now = Date.now();
    const fresh = gameIds.filter((id) => now - (finishedAt.get(id) ?? 0) > LATE_MARK_WINDOW_MS);
    if (fresh.length === 0) return;
    set((state) => ({ pending: new Set([...state.pending, ...fresh]) }));
    for (const id of fresh) {
      clearTimeout(timers.get(id));
      timers.set(
        id,
        setTimeout(() => get().finish(id), ARTWORK_LOADING_TIMEOUT_MS),
      );
    }
  },
  finish: (gameId) => {
    finishedAt.set(gameId, Date.now());
    clearTimeout(timers.get(gameId));
    timers.delete(gameId);
    set((state) => {
      if (!state.pending.has(gameId)) return state;
      const next = new Set(state.pending);
      next.delete(gameId);
      return { pending: next };
    });
  },
}));

/** True while the game's artwork lookup is running. */
export function useIsArtworkLoading(gameId: number): boolean {
  return useArtworkLoading((state) => state.pending.has(gameId));
}
