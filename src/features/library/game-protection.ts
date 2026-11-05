import type { CloudState } from "@/lib/bindings/CloudState";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";

/** What the protection indicator and the dashboard know about one game's saves. */
export interface GameProtection {
  /** The newest snapshot has an archive on this PC. */
  local: boolean;
  /** The newest snapshot is in Drive (`true`), on its way (`"pending"`), or not there. */
  cloud: boolean | "pending" | "conflict";
  snapshotCount: number;
}

export const NO_PROTECTION: GameProtection = { local: false, cloud: false, snapshotCount: 0 };

const IN_FLIGHT: readonly CloudState[] = ["queued", "uploading"];

/** Per-game protection from every snapshot (newest first) and the games with an open conflict. */
export function protectionByGame(
  snapshots: readonly SnapshotSummary[],
  conflictGameIds: ReadonlySet<number> = new Set(),
): Map<number, GameProtection> {
  const newestFirst = [...snapshots].sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  const result = new Map<number, GameProtection>();
  for (const snapshot of newestFirst) {
    const known = result.get(snapshot.gameId);
    if (known) {
      known.snapshotCount += 1;
      continue;
    }
    result.set(snapshot.gameId, {
      local: snapshot.archivePath !== null && snapshot.status !== "missing_local",
      cloud: snapshot.cloud === "synced" ? true : IN_FLIGHT.includes(snapshot.cloud) ? "pending" : false,
      snapshotCount: 1,
    });
  }
  for (const gameId of conflictGameIds) {
    const known = result.get(gameId);
    result.set(gameId, { ...(known ?? NO_PROTECTION), cloud: "conflict" });
  }
  return result;
}

/** Totals for the dashboard orb: snapshots overall, on this PC, in Drive and still queued. */
export function protectionTotals(snapshots: readonly SnapshotSummary[]) {
  return {
    total: snapshots.length,
    onPc: snapshots.filter((snapshot) => snapshot.archivePath !== null && snapshot.status !== "missing_local").length,
    inDrive: snapshots.filter((snapshot) => snapshot.cloud === "synced").length,
    queued: snapshots.filter((snapshot) => IN_FLIGHT.includes(snapshot.cloud)).length,
  };
}
