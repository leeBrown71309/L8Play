import type { ImportPlan } from "@/lib/bindings/ImportPlan";
import type { PlannedGame } from "@/lib/bindings/PlannedGame";
import type { PlannedSnapshot } from "@/lib/bindings/PlannedSnapshot";

/** A backup folder shaped like a real hand-made "Save Games" folder, for the browser preview only. */
function snapshot(label: string | null, root: string, files: number, bytes: number): PlannedSnapshot {
  return { label, root, files, bytes, modifiedAt: "2025-06-02T18:20:00Z", hint: null, hintFile: null };
}

const BASE = "D:\\Backups\\Save Games";

function game(
  folderName: string,
  title: string,
  overrides: Partial<PlannedGame> & Pick<PlannedGame, "snapshots" | "layout">,
): PlannedGame {
  return {
    key: folderName,
    folder: `${BASE}\\${folderName}`,
    folderName,
    guessedTitle: title,
    suggestedTitle: title,
    altTitles: [],
    fromCollection: null,
    duplicateOf: null,
    skippedFiles: 0,
    emptySnapshots: [],
    ...overrides,
  };
}

export function samplePlan(): ImportPlan {
  return {
    source: BASE,
    mode: "games_folder",
    warnings: ["Folders with nothing to import were skipped: save"],
    games: [
      game("Stellar Blade", "Stellar Blade", {
        layout: "variants",
        snapshots: [
          snapshot("after ending A", `${BASE}\\Stellar Blade\\after ending A`, 6, 4_200_000),
          snapshot("after ending B", `${BASE}\\Stellar Blade\\after ending B`, 6, 4_300_000),
          snapshot("before final choice", `${BASE}\\Stellar Blade\\before final choice`, 6, 4_100_000),
        ],
      }),
      game("EldenRing", "Elden Ring", {
        layout: "single",
        snapshots: [snapshot(null, `${BASE}\\EldenRing`, 7, 28_000_000)],
      }),
      game("DSR", "DSR", {
        layout: "single",
        suggestedTitle: null,
        altTitles: ["DARK SOULS REMASTERED"],
        skippedFiles: 2,
        snapshots: [snapshot(null, `${BASE}\\DSR`, 1, 1_200_000)],
      }),
    ],
  };
}
