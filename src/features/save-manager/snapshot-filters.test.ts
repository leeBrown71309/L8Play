import { describe, expect, it } from "vitest";

import { filterSnapshots, isInDrive, isOnPc, matchesStored } from "@/features/save-manager/snapshot-filters";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";

const NOW = Date.parse("2026-11-04T12:00:00Z");

function snapshot(overrides: Partial<SnapshotSummary>): SnapshotSummary {
  return {
    id: "s",
    gameId: 1,
    gameTitle: "Game",
    name: "",
    description: null,
    kind: "auto_exit",
    status: "ready",
    createdAt: "2026-11-03T12:00:00Z",
    verifiedAt: null,
    archivePath: "C:/snapshots/s.zip",
    archiveSizeBytes: 1,
    totalFiles: 1,
    totalBytes: 1,
    archiveSha256: "",
    playtimeSeconds: null,
    pinned: false,
    deviceName: "PC",
    cloud: "none",
    label: null,
    imported: false,
    ...overrides,
  };
}

describe("storage place", () => {
  const both = snapshot({ cloud: "synced" });
  const pcOnly = snapshot({ cloud: "queued" });
  const driveOnly = snapshot({ cloud: "synced", status: "missing_local", archivePath: null });

  it("knows where each copy lives", () => {
    expect(isOnPc(both) && isInDrive(both)).toBe(true);
    expect(isOnPc(pcOnly) && !isInDrive(pcOnly)).toBe(true);
    expect(!isOnPc(driveOnly) && isInDrive(driveOnly)).toBe(true);
  });

  it("filters by place", () => {
    expect(matchesStored(both, "pc_only")).toBe(false);
    expect(matchesStored(pcOnly, "pc_only")).toBe(true);
    expect(matchesStored(driveOnly, "drive_only")).toBe(true);
    expect(matchesStored(both, "drive")).toBe(true);
    expect(matchesStored(driveOnly, "pc")).toBe(false);
  });
});

describe("filterSnapshots", () => {
  const list = [
    snapshot({ id: "a", kind: "manual" }),
    snapshot({ id: "b", kind: "auto_exit", gameId: 2 }),
    snapshot({ id: "c", kind: "safety", createdAt: "2026-09-01T12:00:00Z" }),
  ];

  it("filters by game and kind", () => {
    const filters = { gameId: null, kind: "all", stored: "any", date: "all" } as const;
    expect(filterSnapshots(list, { ...filters, gameId: 2 }, NOW).map((s) => s.id)).toEqual(["b"]);
    expect(filterSnapshots(list, { ...filters, kind: "safety" }, NOW).map((s) => s.id)).toEqual(["c"]);
    expect(filterSnapshots(list, { ...filters, kind: "automatic" }, NOW).map((s) => s.id)).toEqual(["b"]);
  });

  it("filters by age", () => {
    const filters = { gameId: null, kind: "all", stored: "any", date: "30" } as const;
    expect(filterSnapshots(list, filters, NOW).map((s) => s.id)).toEqual(["a", "b"]);
  });
});
