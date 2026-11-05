import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ARTWORK_LOADING_TIMEOUT_MS, useArtworkLoading } from "@/features/library/artwork-loading-store";

describe("artwork loading store", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useArtworkLoading.setState({ pending: new Set() });
  });
  afterEach(() => vi.useRealTimers());

  it("marks games pending and clears them when the fetch finishes", () => {
    useArtworkLoading.getState().markPending([1, 2]);
    expect([...useArtworkLoading.getState().pending]).toEqual([1, 2]);
    useArtworkLoading.getState().finish(1);
    expect([...useArtworkLoading.getState().pending]).toEqual([2]);
  });

  it("gives up after the timeout so a card never shimmers forever", () => {
    useArtworkLoading.getState().markPending([11]);
    vi.advanceTimersByTime(ARTWORK_LOADING_TIMEOUT_MS + 1);
    expect(useArtworkLoading.getState().pending.has(11)).toBe(false);
  });

  it("ignores a mark that arrives right after the game's fetch already finished", () => {
    useArtworkLoading.getState().finish(21);
    useArtworkLoading.getState().markPending([21]);
    expect(useArtworkLoading.getState().pending.has(21)).toBe(false);
  });
});
