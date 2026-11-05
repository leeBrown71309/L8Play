import { renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AUTOPLAY_INTERVAL_MS, nextPage, useCarouselAutoplay } from "@/features/dashboard/use-carousel-autoplay";

function mockReducedMotion(reduced: boolean) {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: reduced && query.includes("reduce"),
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
  }));
}

describe("nextPage", () => {
  it("wraps around and stays on 0 for one or no page", () => {
    expect(nextPage(0, 4)).toBe(1);
    expect(nextPage(3, 4)).toBe(0);
    expect(nextPage(0, 1)).toBe(0);
    expect(nextPage(0, 0)).toBe(0);
  });
});

describe("useCarouselAutoplay", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockReducedMotion(false);
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("advances after the interval and restarts the countdown on each page", () => {
    const onAdvance = vi.fn();
    const { rerender } = renderHook((props) => useCarouselAutoplay(props), {
      initialProps: { count: 4, page: 0, paused: false, onAdvance },
    });
    vi.advanceTimersByTime(AUTOPLAY_INTERVAL_MS - 1);
    expect(onAdvance).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(onAdvance).toHaveBeenCalledWith(1);

    rerender({ count: 4, page: 1, paused: false, onAdvance });
    vi.advanceTimersByTime(AUTOPLAY_INTERVAL_MS);
    expect(onAdvance).toHaveBeenLastCalledWith(2);
  });

  it("does not run while paused or with a single page", () => {
    const onAdvance = vi.fn();
    renderHook(() => useCarouselAutoplay({ count: 4, page: 0, paused: true, onAdvance }));
    renderHook(() => useCarouselAutoplay({ count: 1, page: 0, paused: false, onAdvance }));
    vi.advanceTimersByTime(AUTOPLAY_INTERVAL_MS * 3);
    expect(onAdvance).not.toHaveBeenCalled();
  });

  it("does not run when the user prefers reduced motion", () => {
    mockReducedMotion(true);
    const onAdvance = vi.fn();
    renderHook(() => useCarouselAutoplay({ count: 4, page: 0, paused: false, onAdvance }));
    vi.advanceTimersByTime(AUTOPLAY_INTERVAL_MS * 3);
    expect(onAdvance).not.toHaveBeenCalled();
  });
});
