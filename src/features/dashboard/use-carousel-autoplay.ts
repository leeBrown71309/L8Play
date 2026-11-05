import { useEffect, useSyncExternalStore } from "react";

/** Time each game stays on the hero card before the next one slides in. */
export const AUTOPLAY_INTERVAL_MS = 7000;

/** The page after `page`, wrapping around; always 0 when there is nothing to cycle through. */
export function nextPage(page: number, count: number): number {
  return count <= 1 ? 0 : (page + 1) % count;
}

function subscribeMedia(query: string, onChange: () => void) {
  const media = window.matchMedia(query);
  media.addEventListener("change", onChange);
  return () => media.removeEventListener("change", onChange);
}

function usePrefersReducedMotion(): boolean {
  const query = "(prefers-reduced-motion: reduce)";
  return useSyncExternalStore(
    (onChange) => subscribeMedia(query, onChange),
    () => window.matchMedia(query).matches,
    () => false,
  );
}

function useDocumentHidden(): boolean {
  return useSyncExternalStore(
    (onChange) => {
      document.addEventListener("visibilitychange", onChange);
      return () => document.removeEventListener("visibilitychange", onChange);
    },
    () => document.hidden,
    () => false,
  );
}

interface AutoplayOptions {
  count: number;
  page: number;
  /** True while the pointer or keyboard focus is on the carousel. */
  paused: boolean;
  onAdvance: (page: number) => void;
  intervalMs?: number;
}

/**
 * Advances a carousel on a timer. The countdown restarts on every page change (manual or not),
 * and does not run while paused, while the window is hidden, or when the user prefers reduced motion.
 */
export function useCarouselAutoplay({
  count,
  page,
  paused,
  onAdvance,
  intervalMs = AUTOPLAY_INTERVAL_MS,
}: AutoplayOptions): void {
  const reducedMotion = usePrefersReducedMotion();
  const hidden = useDocumentHidden();
  const running = count > 1 && !paused && !reducedMotion && !hidden;

  useEffect(() => {
    if (!running) return;
    const timer = window.setTimeout(() => onAdvance(nextPage(page, count)), intervalMs);
    return () => window.clearTimeout(timer);
  }, [running, page, count, intervalMs, onAdvance]);
}
