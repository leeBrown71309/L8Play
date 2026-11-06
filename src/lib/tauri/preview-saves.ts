import type { SaveLocation } from "@/lib/bindings/SaveLocation";
import type { SnapshotKind } from "@/lib/bindings/SnapshotKind";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";

/**
 * Sample save data for the browser preview only (game 1 = Sekiro in the sample library).
 * Real data comes from Ludusavi detection and the snapshot engine.
 */
const hoursAgo = (hours: number) => new Date(Date.now() - hours * 3_600_000).toISOString();

export function sampleLocations(gameId: number): SaveLocation[] {
  if (gameId !== 1) return [];
  return [
    {
      id: 1,
      gameId,
      path: "C:\\Users\\Ryu\\AppData\\Roaming\\Sekiro\\76561198041122907",
      kind: "directory",
      source: "ludusavi",
      exists: true,
    },
  ];
}

function snapshot(id: number, hours: number, kind: SnapshotKind, name: string, pinned = false): SnapshotSummary {
  return {
    id: `preview-${id}`,
    gameId: 1,
    gameTitle: "Sekiro: Shadows Die Twice",
    name,
    description: name.startsWith("Before Isshin") ? "Final fight. All prayer beads, Mortal Blade." : null,
    kind,
    // Sample cloud states: the oldest snapshot lives only in Drive, two others are synced or waiting.
    status: id === 1 ? "missing_local" : "ready",
    createdAt: hoursAgo(hours),
    verifiedAt: hoursAgo(hours),
    archivePath: id === 1 ? null : `C:\\Users\\Ryu\\Documents\\L8Play\\Snapshots\\sekiro\\preview-${id}.zip`,
    archiveSizeBytes: 3_100_000,
    totalFiles: 1,
    totalBytes: 10_276_000,
    archiveSha256: "4f1c0000000000000000000000000000000000000000000000000000000a9a2e",
    playtimeSeconds: Math.round((64 - hours / 10) * 3600),
    pinned,
    deviceName: id === 1 ? "LAPTOP-RYU" : "DESKTOP-L8",
    cloud: id === 1 || id === 5 ? "synced" : id === 4 ? "queued" : "none",
    label: null,
    imported: false,
  };
}

export function sampleSnapshots(gameId: number | null): SnapshotSummary[] {
  if (gameId !== null && gameId !== 1) return [];
  return [
    snapshot(5, 0.5, "auto_exit", ""),
    snapshot(4, 1.2, "manual", "Before Isshin, the Sword Saint", true),
    snapshot(3, 22, "auto_exit", ""),
    snapshot(2, 72, "safety", ""),
    snapshot(1, 120, "manual", "Owl (Father) — Hirata Estate"),
  ];
}
