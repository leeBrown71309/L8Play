import { describe, expect, it } from "vitest";

import {
  buildRow,
  isImportable,
  setLabel,
  toRequests,
  totalSnapshots,
  withReplannedGame,
} from "@/features/saves-import/review-state";
import type { PlannedGame } from "@/lib/bindings/PlannedGame";

function snapshot(label: string | null) {
  return { label, root: `D:\\b\\${label ?? "x"}`, files: 3, bytes: 100, modifiedAt: null, hint: null, hintFile: null };
}

function game(overrides: Partial<PlannedGame> = {}): PlannedGame {
  return {
    key: "Stellar Blade",
    folder: "D:\\b\\Stellar Blade",
    folderName: "Stellar Blade",
    guessedTitle: "Stellar Blade",
    suggestedTitle: null,
    altTitles: [],
    layout: "variants",
    snapshots: [snapshot("after ending A"), snapshot("after ending B")],
    fromCollection: null,
    duplicateOf: null,
    skippedFiles: 0,
    emptySnapshots: [],
    ...overrides,
  };
}

describe("import review", () => {
  it("starts from the database name when one matched", () => {
    expect(buildRow(game({ suggestedTitle: "STELLAR BLADE" })).title).toBe("STELLAR BLADE");
    expect(buildRow(game()).title).toBe("Stellar Blade");
  });

  it("keeps the folder names as snapshot names and sends edited ones", () => {
    const row = setLabel(buildRow(game()), 1, "  true ending  ");
    const [request] = toRequests([row]);
    expect(request?.snapshots.map((s) => s.label)).toEqual(["after ending A", "true ending"]);
  });

  it("clears a name left empty", () => {
    const row = setLabel(buildRow(game()), 0, "   ");
    expect(toRequests([row])[0]?.snapshots[0]?.label).toBeNull();
  });

  it("skips unticked rows and rows without a title", () => {
    const off = { ...buildRow(game()), include: false };
    const untitled = { ...buildRow(game()), title: "  " };
    expect(isImportable(off)).toBe(false);
    expect(isImportable(untitled)).toBe(false);
    expect(toRequests([off, untitled, buildRow(game())])).toHaveLength(1);
    expect(totalSnapshots([off, buildRow(game())])).toBe(2);
  });

  it("keeps the title and choice of the user when the layout is read again", () => {
    const row = { ...buildRow(game()), title: "My title", include: false };
    const next = withReplannedGame(row, game({ layout: "single", snapshots: [snapshot(null)] }));
    expect(next.title).toBe("My title");
    expect(next.include).toBe(false);
    expect(next.labels).toEqual([""]);
  });
});
