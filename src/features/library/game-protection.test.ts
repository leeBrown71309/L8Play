import { describe, expect, it } from "vitest";

import { protectionByGame, protectionTotals } from "@/features/library/game-protection";
import type { CloudState } from "@/lib/bindings/CloudState";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";

const snapshot = (
  id: string,
  gameId: number,
  createdAt: string,
  cloud: CloudState,
  archivePath: string | null = "a.zip",
): SnapshotSummary =>
  ({ id, gameId, createdAt, cloud, archivePath, status: archivePath ? "ready" : "missing_local" }) as SnapshotSummary;

describe("protectionByGame", () => {
  it("reads the newest snapshot and counts them all", () => {
    const map = protectionByGame([
      snapshot("old", 1, "2026-11-01T10:00:00Z", "synced"),
      snapshot("new", 1, "2026-11-03T10:00:00Z", "uploading"),
    ]);
    expect(map.get(1)).toEqual({ local: true, cloud: "pending", snapshotCount: 2 });
  });

  it("marks a Drive-only copy as not on this PC", () => {
    expect(protectionByGame([snapshot("s", 2, "2026-11-01T10:00:00Z", "synced", null)]).get(2)).toEqual({
      local: false,
      cloud: true,
      snapshotCount: 1,
    });
  });

  it("flags open conflicts, even for a game with no snapshot", () => {
    const map = protectionByGame([snapshot("s", 1, "2026-11-01T10:00:00Z", "synced")], new Set([1, 9]));
    expect(map.get(1)?.cloud).toBe("conflict");
    expect(map.get(9)).toEqual({ local: false, cloud: "conflict", snapshotCount: 0 });
  });
});

describe("protectionTotals", () => {
  it("counts snapshots per place and the queue", () => {
    const totals = protectionTotals([
      snapshot("a", 1, "2026-11-01T10:00:00Z", "synced"),
      snapshot("b", 1, "2026-11-02T10:00:00Z", "queued"),
      snapshot("c", 2, "2026-11-02T10:00:00Z", "none", null),
    ]);
    expect(totals).toEqual({ total: 3, onPc: 2, inDrive: 1, queued: 1 });
  });
});
