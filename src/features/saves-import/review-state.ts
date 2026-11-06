import type { ImportGameRequest } from "@/lib/bindings/ImportGameRequest";
import type { ImportPlan } from "@/lib/bindings/ImportPlan";
import type { PlannedGame } from "@/lib/bindings/PlannedGame";
import type { PlannedSnapshot } from "@/lib/bindings/PlannedSnapshot";

/** One game of the review step, as the user is editing it. */
export interface ReviewRow {
  game: PlannedGame;
  include: boolean;
  title: string;
  /** Edited snapshot names, one per planned snapshot (empty string = unnamed). */
  labels: string[];
}

/** The title to start from: the game database name when it matched, else the guess from the folder. */
export function startingTitle(game: PlannedGame): string {
  return game.suggestedTitle ?? game.guessedTitle;
}

export function buildRow(game: PlannedGame): ReviewRow {
  return {
    game,
    include: true,
    title: startingTitle(game),
    labels: game.snapshots.map((snapshot) => snapshot.label ?? ""),
  };
}

export function buildRows(plan: ImportPlan): ReviewRow[] {
  return plan.games.map(buildRow);
}

/** Replaces a row game (after the layout was read again) while keeping the title and choice of the user. */
export function withReplannedGame(row: ReviewRow, game: PlannedGame): ReviewRow {
  return { ...buildRow(game), include: row.include, title: row.title };
}

export function setLabel(row: ReviewRow, index: number, label: string): ReviewRow {
  return { ...row, labels: row.labels.map((current, position) => (position === index ? label : current)) };
}

/** A row can be imported when it is ticked and has a title. */
export function isImportable(row: ReviewRow): boolean {
  return row.include && row.title.trim() !== "";
}

export function snapshotsToImport(row: ReviewRow): PlannedSnapshot[] {
  return row.game.snapshots.map((snapshot, index) => {
    const label = (row.labels[index] ?? "").trim();
    return { ...snapshot, label: label === "" ? null : label };
  });
}

export function toRequests(rows: ReviewRow[]): ImportGameRequest[] {
  return rows.filter(isImportable).map((row) => ({ title: row.title.trim(), snapshots: snapshotsToImport(row) }));
}

export function totalSnapshots(rows: ReviewRow[]): number {
  return rows.filter(isImportable).reduce((sum, row) => sum + row.game.snapshots.length, 0);
}
