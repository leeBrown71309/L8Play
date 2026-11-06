import { describe, expect, it } from "vitest";

import { filterConsoleGames, letterOf, lettersInUse, stepFilter } from "@/features/console/core/console-filter";
import type { GameSummary } from "@/lib/bindings/GameSummary";

function game(id: number, title: string, patch: Partial<GameSummary> = {}): GameSummary {
  return {
    id,
    title,
    platform: "steam",
    platformGameId: null,
    installed: true,
    archived: false,
    favorite: false,
    lastPlayedAt: null,
    playtimeSeconds: 0,
    sizeBytes: null,
    installPath: null,
    artwork: { cover: null, hero: null, logo: null, header: null, icon: null },
    ...patch,
  };
}

const GAMES = [
  game(1, "Alan Wake 2", { lastPlayedAt: "2026-11-01T10:00:00Z" }),
  game(2, "Élan Vital", { favorite: true }),
  game(3, "7 Days", { installed: false }),
  game(4, "Hades II", { lastPlayedAt: "2026-11-03T10:00:00Z", favorite: true }),
];

describe("console filter", () => {
  it("files titles under an accent-free letter, with # for digits and symbols", () => {
    expect(letterOf("Élan Vital")).toBe("E");
    expect(letterOf("  hades")).toBe("H");
    expect(letterOf("7 Days")).toBe("#");
    expect(letterOf("¡Hola!")).toBe("#");
    expect(letterOf("")).toBe("#");
  });

  it("applies each chip", () => {
    expect(filterConsoleGames(GAMES, "all", null)).toHaveLength(4);
    expect(filterConsoleGames(GAMES, "recent", null).map((entry) => entry.id)).toEqual([4, 1]);
    expect(
      filterConsoleGames(GAMES, "favourites", null)
        .map((entry) => entry.id)
        .sort(),
    ).toEqual([2, 4]);
    expect(filterConsoleGames(GAMES, "installed", null)).toHaveLength(3);
    expect(filterConsoleGames(GAMES, "not-installed", null).map((entry) => entry.id)).toEqual([3]);
  });

  it("narrows by starting letter on top of a chip", () => {
    expect(filterConsoleGames(GAMES, "all", "H").map((entry) => entry.id)).toEqual([4]);
    expect(filterConsoleGames(GAMES, "all", "#").map((entry) => entry.id)).toEqual([3]);
    expect(filterConsoleGames(GAMES, "favourites", "A")).toEqual([]);
  });

  it("lists the letters that have games", () => {
    expect([...lettersInUse(GAMES)].sort()).toEqual(["#", "A", "E", "H"]);
  });

  it("steps through the chips and stops at both ends", () => {
    expect(stepFilter("all", -1)).toBe("all");
    expect(stepFilter("all", 1)).toBe("recent");
    expect(stepFilter("not-installed", 1)).toBe("saves-only");
    expect(stepFilter("saves-only", 1)).toBe("saves-only");
  });
});
