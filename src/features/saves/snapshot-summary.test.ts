import { describe, expect, it } from "vitest";

import { countByFilter, filterSnapshots, summarizeCopies } from "@/features/saves/snapshot-summary";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";

function snapshot(overrides: Partial<SnapshotSummary>): SnapshotSummary {
  return {
    id: "s",
    gameId: 1,
    gameTitle: "Sekiro",
    name: "",
    description: null,
    kind: "auto_exit",
    status: "ready",
    createdAt: "2026-11-04T21:12:00Z",
    verifiedAt: null,
    archivePath: "a.zip",
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

const SAMPLE = [
  snapshot({ id: "a", kind: "manual", name: "Before Isshin", cloud: "synced" }),
  snapshot({ id: "b", kind: "auto_exit", cloud: "synced" }),
  snapshot({ id: "c", kind: "scheduled", status: "missing_local", cloud: "synced" }),
  snapshot({ id: "d", kind: "safety", description: "Owl fight" }),
];

describe("snapshot summary", () => {
  it("counts copies on this PC and in Drive", () => {
    expect(summarizeCopies(SAMPLE)).toEqual({ total: 4, onPc: 3, inDrive: 3 });
  });

  it("counts every automatic kind under Auto", () => {
    expect(countByFilter(SAMPLE)).toEqual({ all: 4, manual: 1, auto: 2, safety: 1 });
  });

  it("filters by chip and by text in the name or the note", () => {
    const name = (item: SnapshotSummary) => item.name || "Game closed";
    expect(filterSnapshots(SAMPLE, "manual", "", name).map((item) => item.id)).toEqual(["a"]);
    expect(filterSnapshots(SAMPLE, "all", "owl", name).map((item) => item.id)).toEqual(["d"]);
    expect(filterSnapshots(SAMPLE, "auto", "closed", name).map((item) => item.id)).toEqual(["b", "c"]);
  });
});
