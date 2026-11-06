import type { GameSummary } from "@/lib/bindings/GameSummary";

/** What the main button of a game does. Saves-only games have nothing to launch or install. */
export type PrimaryAction = "play" | "install" | "saves";

export function primaryAction(game: Pick<GameSummary, "installed" | "archived">): PrimaryAction {
  if (game.archived) return "saves";
  return game.installed ? "play" : "install";
}

/** `true` when L8Play only manages this game's saves (imported from an older backup). */
export function isSavesOnly(game: Pick<GameSummary, "archived">): boolean {
  return game.archived;
}
