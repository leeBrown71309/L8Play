import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";

export type SnapshotFilter = "all" | "manual" | "auto" | "safety";

export interface CopySummary {
  total: number;
  /** Snapshots whose archive is on this PC. */
  onPc: number;
  /** Snapshots with a verified copy in Google Drive. */
  inDrive: number;
}

/** Where a game's snapshots live. */
export function summarizeCopies(snapshots: readonly SnapshotSummary[]): CopySummary {
  return {
    total: snapshots.length,
    onPc: snapshots.filter((snapshot) => snapshot.status !== "missing_local").length,
    inDrive: snapshots.filter((snapshot) => snapshot.cloud === "synced").length,
  };
}

/** The Saves-tab chip a snapshot belongs to: manual, safety copies, or everything automatic. */
export function filterOf(snapshot: SnapshotSummary): Exclude<SnapshotFilter, "all"> {
  if (snapshot.kind === "manual") return "manual";
  if (snapshot.kind === "safety") return "safety";
  return "auto";
}

/** Counts for the filter chips. */
export function countByFilter(snapshots: readonly SnapshotSummary[]): Record<SnapshotFilter, number> {
  const counts: Record<SnapshotFilter, number> = { all: snapshots.length, manual: 0, auto: 0, safety: 0 };
  for (const snapshot of snapshots) counts[filterOf(snapshot)] += 1;
  return counts;
}

/** Snapshots matching a filter chip and a free-text query on name, note and game. */
export function filterSnapshots(
  snapshots: readonly SnapshotSummary[],
  filter: SnapshotFilter,
  query: string,
  displayName: (snapshot: SnapshotSummary) => string,
): SnapshotSummary[] {
  const needle = query.trim().toLowerCase();
  return snapshots.filter((snapshot) => {
    if (filter !== "all" && filterOf(snapshot) !== filter) return false;
    if (needle === "") return true;
    return [displayName(snapshot), snapshot.description ?? "", snapshot.gameTitle].some((text) =>
      text.toLowerCase().includes(needle),
    );
  });
}
