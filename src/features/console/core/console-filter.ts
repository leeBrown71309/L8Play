import { filterGames, recentlyPlayed } from "@/features/library/filter-games";
import type { LibraryFilters } from "@/features/library/library-view-store";
import type { GameSummary } from "@/lib/bindings/GameSummary";

/** The chips above the coverflow. */
export const CONSOLE_FILTERS = ["all", "recent", "favourites", "installed", "not-installed"] as const;
export type ConsoleFilter = (typeof CONSOLE_FILTERS)[number];

/** "#" stands for every title that starts with a digit or a symbol. */
export const LETTERS = [..."ABCDEFGHIJKLMNOPQRSTUVWXYZ", "#"] as const;
export type Letter = (typeof LETTERS)[number];

const RECENT_LIMIT = 40;

const BASE: LibraryFilters = {
  search: "",
  platform: "all",
  install: "all",
  favoritesOnly: false,
  sort: "recent",
};

/** The letter a title is filed under, ignoring accents and case ("Élan" → "E", "7 Days" → "#"). */
export function letterOf(title: string): Letter {
  const first = title
    .trim()
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .charAt(0)
    .toUpperCase();
  return (LETTERS as readonly string[]).includes(first) && first !== "#" ? (first as Letter) : "#";
}

/** Games for a filter chip, optionally narrowed to one starting letter. */
export function filterConsoleGames(games: readonly GameSummary[], filter: ConsoleFilter, letter: Letter | null) {
  let result: GameSummary[];
  switch (filter) {
    case "recent":
      result = recentlyPlayed(games, RECENT_LIMIT);
      break;
    case "favourites":
      result = filterGames(games, { ...BASE, favoritesOnly: true });
      break;
    case "installed":
      result = filterGames(games, { ...BASE, install: "installed" });
      break;
    case "not-installed":
      result = filterGames(games, { ...BASE, install: "not-installed" });
      break;
    default:
      result = filterGames(games, BASE);
  }
  return letter === null ? result : result.filter((game) => letterOf(game.title) === letter);
}

/** Letters that have at least one game, so the picker can dim the empty ones. */
export function lettersInUse(games: readonly GameSummary[]): Set<Letter> {
  return new Set(games.map((game) => letterOf(game.title)));
}

/** Next filter chip in a direction, stopping at both ends. */
export function stepFilter(current: ConsoleFilter, delta: number): ConsoleFilter {
  const index = CONSOLE_FILTERS.indexOf(current) + delta;
  return CONSOLE_FILTERS[Math.min(CONSOLE_FILTERS.length - 1, Math.max(0, index))] ?? current;
}
