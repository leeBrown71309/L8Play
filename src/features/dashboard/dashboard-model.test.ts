import { describe, expect, it } from "vitest";

import { activeUploads, buildAttention, transferPercent } from "@/features/dashboard/dashboard-model";
import type { GameSaveStatus } from "@/lib/bindings/GameSaveStatus";
import type { SyncOperationInfo } from "@/lib/bindings/SyncOperationInfo";
import { sampleLibrary } from "@/lib/tauri/preview-library";

const games = sampleLibrary();
const status = (gameId: number, snapshotCount: number, locationCount = 1): GameSaveStatus => ({
  gameId,
  locationCount,
  snapshotCount,
  lastSnapshotAt: null,
  unknownToLudusavi: false,
});
const operation = (gameTitle: string, patch: Partial<SyncOperationInfo> = {}): SyncOperationInfo => ({
  id: 1,
  snapshotId: "s",
  gameTitle,
  snapshotName: "Game closed",
  direction: "upload",
  status: "running",
  attempts: 0,
  bytesDone: 50,
  bytesTotal: 200,
  lastError: null,
  nextAttemptAt: null,
  ...patch,
});

describe("buildAttention", () => {
  it("lists uploads before games that are not protected, most played first", () => {
    const [first, second] = games.filter((game) => game.installed);
    if (!first || !second) throw new Error("sample library is too small");
    const items = buildAttention({
      games,
      statuses: [status(first.id, 0, 0), status(second.id, 0)],
      conflicts: [],
      operations: [operation(second.title)],
      limit: 5,
    });
    expect(items[0]?.kind).toBe("uploading");
    expect(items[0]?.game.id).toBe(second.id);
    expect(items.find((item) => item.game.id === first.id)?.kind).toBe("unprotected");
  });

  it("separates 'no save found' from 'no snapshot yet' and honours the limit", () => {
    const installed = games.filter((game) => game.installed).slice(0, 3);
    const items = buildAttention({
      games,
      statuses: [
        status(installed[0]?.id ?? 0, 0, 0),
        status(installed[1]?.id ?? 0, 0, 2),
        status(installed[2]?.id ?? 0, 0),
      ],
      conflicts: [],
      operations: [],
      limit: 2,
    });
    expect(items).toHaveLength(2);
    expect(new Set(items.map((item) => item.kind)).size).toBeGreaterThan(0);
  });

  it("ignores protected games", () => {
    const protectedStatuses = games.map((game) => status(game.id, 3));
    expect(buildAttention({ games, statuses: protectedStatuses, conflicts: [], operations: [], limit: 5 })).toEqual([]);
  });
});

describe("transfers", () => {
  it("computes a percentage that never exceeds 100", () => {
    expect(transferPercent(50, 200)).toBe(25);
    expect(transferPercent(500, 200)).toBe(100);
    expect(transferPercent(0, 0)).toBe(0);
  });

  it("shows running uploads before queued ones", () => {
    const list = activeUploads(
      [
        operation("A", { status: "queued", id: 1 }),
        operation("B", { id: 2 }),
        operation("C", { status: "failed", id: 3 }),
      ],
      5,
    );
    expect(list.map((entry) => entry.gameTitle)).toEqual(["B", "A"]);
  });
});
