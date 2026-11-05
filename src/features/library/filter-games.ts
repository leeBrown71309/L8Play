import type { InstallFilter, LibraryFilters } from "@/features/library/library-view-store";
import type { GameSummary } from "@/lib/bindings/GameSummary";

/** Case- and accent-insensitive comparison key. */
function normalize(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
}

const byTitle = (a: GameSummary, b: GameSummary) => a.title.localeCompare(b.title, undefined, { sensitivity: "base" });

const COMPARATORS: Record<LibraryFilters["sort"], (a: GameSummary, b: GameSummary) => number> = {
  // Never-played games go last, alphabetically.
  recent: (a, b) => (b.lastPlayedAt ?? "").localeCompare(a.lastPlayedAt ?? "") || byTitle(a, b),
  title: byTitle,
  playtime: (a, b) => b.playtimeSeconds - a.playtimeSeconds || byTitle(a, b),
  size: (a, b) => (b.sizeBytes ?? 0) - (a.sizeBytes ?? 0) || byTitle(a, b),
  // Ids grow with every game added, so the highest id is the newest.
  added: (a, b) => b.id - a.id,
};

function matchesInstall(game: GameSummary, install: InstallFilter): boolean {
  return install === "all" || (install === "installed" ? game.installed : !game.installed);
}

export function filterGames(games: readonly GameSummary[], filters: LibraryFilters): GameSummary[] {
  const query = normalize(filters.search.trim());
  return games
    .filter((game) => filters.platform === "all" || game.platform === filters.platform)
    .filter((game) => matchesInstall(game, filters.install))
    .filter((game) => !filters.favoritesOnly || game.favorite)
    .filter((game) => query === "" || normalize(game.title).includes(query))
    .sort(COMPARATORS[filters.sort]);
}

/** Most recently played games first (only games with a play date). */
export function recentlyPlayed(games: readonly GameSummary[], limit: number): GameSummary[] {
  return games
    .filter((game) => game.lastPlayedAt !== null)
    .sort(COMPARATORS.recent)
    .slice(0, limit);
}

/** How many games each install filter would list, given the other filters (so the chips stay honest). */
export function countByInstall(games: readonly GameSummary[], filters: LibraryFilters): Record<InstallFilter, number> {
  const matching = filterGames(games, { ...filters, install: "all" });
  const installed = matching.filter((game) => game.installed).length;
  return { all: matching.length, installed, "not-installed": matching.length - installed };
}
