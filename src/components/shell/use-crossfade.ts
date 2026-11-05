import { useEffect, useState } from "react";

/** How long the outgoing layer stays mounted under the incoming one. */
export const CROSSFADE_MS = 700;

interface Crossfade<T> {
  /** What is shown now (fades in whenever it changes). */
  current: T | null;
  /** What was shown just before, kept mounted until the fade has finished. */
  leaving: T | null;
}

/**
 * Keeps the previous item around for the length of a cross-fade, so the outgoing image can stay
 * opaque under the incoming one. `keyOf` decides when the item counts as changed.
 */
export function useCrossfade<T>(item: T | null, keyOf: (value: T) => string | number): Crossfade<T> {
  const [state, setState] = useState<Crossfade<T>>({ current: item, leaving: null });
  const changed =
    (state.current === null) !== (item === null) ||
    (state.current !== null && item !== null && keyOf(state.current) !== keyOf(item));

  // Derived during render: the swap and the first frame of the new layer are one commit.
  if (changed) setState({ current: item, leaving: state.current });

  const { leaving } = state;
  useEffect(() => {
    if (leaving === null) return;
    const timer = window.setTimeout(() => setState((previous) => ({ ...previous, leaving: null })), CROSSFADE_MS);
    return () => window.clearTimeout(timer);
  }, [leaving]);

  return state;
}
