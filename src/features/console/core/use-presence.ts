import { useEffect, useState } from "react";

/**
 * Keeps something mounted while it animates out: `mounted` stays true for `exitMs` after `open` turns false,
 * and `closing` is true during that time so CSS can play the reverse animation.
 */
export function usePresence(open: boolean, exitMs: number): { mounted: boolean; closing: boolean } {
  // `finished` is true once the exit animation has ended (and from the start when nothing is open).
  const [finished, setFinished] = useState(!open);
  const [wasOpen, setWasOpen] = useState(open);

  // Reopening while it is still animating out (or after): the derived state is reset during render.
  if (open !== wasOpen) {
    setWasOpen(open);
    if (open) setFinished(false);
  }

  useEffect(() => {
    if (open) return undefined;
    const timer = window.setTimeout(() => setFinished(true), exitMs);
    return () => window.clearTimeout(timer);
  }, [open, exitMs]);

  return { mounted: open || !finished, closing: !open && !finished };
}
