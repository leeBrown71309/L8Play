import { beforeEach, describe, expect, it } from "vitest";

import { useConsoleGameWindow } from "@/features/console/core/game-window";

describe("console game window handover", () => {
  beforeEach(() => useConsoleGameWindow.getState().reset());

  it("brings the console back only after the launched game started and then exited", () => {
    const { handOver, onSession } = useConsoleGameWindow.getState();
    handOver(7);
    expect(onSession(7, false)).toBe(false);
    expect(onSession(7, true)).toBe(false);
    expect(onSession(7, false)).toBe(true);
  });

  it("ignores games the console did not launch", () => {
    const { handOver, onSession } = useConsoleGameWindow.getState();
    handOver(7);
    onSession(7, true);
    expect(onSession(8, false)).toBe(false);
  });

  it("does nothing when no game was handed over", () => {
    expect(useConsoleGameWindow.getState().onSession(7, false)).toBe(false);
  });
});
