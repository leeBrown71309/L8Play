import { describe, expect, it } from "vitest";

import { isSavesOnly, primaryAction } from "@/features/library/game-availability";

describe("game availability", () => {
  it("plays installed games and installs the others", () => {
    expect(primaryAction({ installed: true, archived: false })).toBe("play");
    expect(primaryAction({ installed: false, archived: false })).toBe("install");
  });

  it("opens the saves of a saves-only game, never launching or installing it", () => {
    expect(primaryAction({ installed: false, archived: true })).toBe("saves");
    expect(isSavesOnly({ archived: true })).toBe(true);
    expect(isSavesOnly({ archived: false })).toBe(false);
  });
});
