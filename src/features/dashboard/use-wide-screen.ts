import { useSyncExternalStore } from "react";

/** The design's wide dashboard (1920 × 1080) starts here; below it the 1366 × 768 layout applies. */
export const WIDE_SCREEN_QUERY = "(min-width: 1700px)";

function subscribe(onChange: () => void): () => void {
  const list = window.matchMedia(WIDE_SCREEN_QUERY);
  list.addEventListener("change", onChange);
  return () => list.removeEventListener("change", onChange);
}

/** True on screens wide enough for six tiles, four attention rows and the recent-backups strip. */
export function useWideScreen(): boolean {
  return useSyncExternalStore(
    subscribe,
    () => window.matchMedia(WIDE_SCREEN_QUERY).matches,
    () => false,
  );
}
