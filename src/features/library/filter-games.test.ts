import { describe, expect, it } from "vitest";

import { countByInstall, filterGames, recentlyPlayed } from "@/features/library/filter-games";
import type { LibraryFilters } from "@/features/library/library-view-store";
import { sampleLibrary } from "@/lib/tauri/preview-library";

const ALL: LibraryFilters = { search: "", platform: "all", install: "all", favoritesOnly: false, sort: "title" };
const games = sampleLibrary();

describe("filterGames", () => {
  it("filters by platform, installation and favourites", () => {
    expect(filterGames(games, { ...ALL, platform: "epic" }).every((game) => game.platform === "epic")).toBe(true);
    expect(filterGames(games, { ...ALL, install: "installed" }).some((game) => !game.installed)).toBe(false);
    expect(filterGames(games, { ...ALL, install: "not-installed" }).every((game) => !game.installed)).toBe(true);
    expect(filterGames(games, { ...ALL, install: "not-installed" }).length).toBeGreaterThan(1);
    expect(filterGames(games, { ...ALL, favoritesOnly: true }).map((game) => game.title)).toEqual([
      "Monster Hunter: World",
      "Sekiro: Shadows Die Twice",
      "Stardew Valley",
    ]);
  });

  it("searches without case or accents", () => {
    expect(filterGames(games, { ...ALL, search: "ÉLDEN" }).map((game) => game.title)).toEqual(["ELDEN RING"]);
  });

  it("sorts by playtime, then title", () => {
    expect(filterGames(games, { ...ALL, sort: "playtime" })[0]?.title).toBe("Minecraft: Java Edition");
  });

  it("sorts by date added, newest first", () => {
    const sorted = filterGames(games, { ...ALL, sort: "added" });
    expect(sorted.map((game) => game.id)).toEqual([...games].map((game) => game.id).sort((a, b) => b - a));
  });

  it("does not mutate its input", () => {
    const copy = [...games];
    filterGames(games, { ...ALL, sort: "size" });
    expect(games).toEqual(copy);
  });
});

describe("countByInstall", () => {
  it("counts each install state under the other filters", () => {
    const all = countByInstall(games, ALL);
    expect(all.all).toBe(games.length);
    expect(all.installed + all["not-installed"]).toBe(games.length);
    const epic = countByInstall(games, { ...ALL, platform: "epic", install: "installed" });
    expect(epic.all).toBe(filterGames(games, { ...ALL, platform: "epic" }).length);
  });
});

describe("recentlyPlayed", () => {
  it("keeps played games, newest first", () => {
    const recent = recentlyPlayed(games, 3);
    expect(recent.map((game) => game.title)).toEqual([
      "Sekiro: Shadows Die Twice",
      "Minecraft Dungeons II",
      "Monster Hunter: World",
    ]);
  });
});
