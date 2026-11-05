import type { SnapshotKind } from "@/lib/bindings/SnapshotKind";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";

export type KindFilter = "all" | "manual" | "automatic" | "safety";
export type StoredFilter = "any" | "pc" | "drive" | "pc_only" | "drive_only";
export type DateFilter = "all" | "7" | "30";

export interface SnapshotFilters {
  gameId: number | null;
  kind: KindFilter;
  stored: StoredFilter;
  date: DateFilter;
}

const KIND_GROUPS: Record<Exclude<KindFilter, "all">, readonly SnapshotKind[]> = {
  manual: ["manual"],
  automatic: ["auto_exit", "auto_launch", "scheduled"],
  safety: ["safety", "conflict_copy"],
};

const DAY_MS = 86_400_000;

/** The archive is on this PC (cloud-only snapshots have to be downloaded first). */
export function isOnPc(snapshot: SnapshotSummary): boolean {
  return snapshot.archivePath !== null && snapshot.status !== "missing_local";
}

export function isInDrive(snapshot: SnapshotSummary): boolean {
  return snapshot.cloud === "synced";
}

export function matchesStored(snapshot: SnapshotSummary, stored: StoredFilter): boolean {
  switch (stored) {
    case "any":
      return true;
    case "pc":
      return isOnPc(snapshot);
    case "drive":
      return isInDrive(snapshot);
    case "pc_only":
      return isOnPc(snapshot) && !isInDrive(snapshot);
    case "drive_only":
      return isInDrive(snapshot) && !isOnPc(snapshot);
  }
}

/** Applies the Save Manager's game, kind, storage-place and date filters. */
export function filterSnapshots(
  snapshots: readonly SnapshotSummary[],
  filters: SnapshotFilters,
  now: number = Date.now(),
): SnapshotSummary[] {
  const cutoff = filters.date === "all" ? 0 : now - Number(filters.date) * DAY_MS;
  return snapshots.filter(
    (snapshot) =>
      (filters.gameId === null || snapshot.gameId === filters.gameId) &&
      (filters.kind === "all" || KIND_GROUPS[filters.kind].includes(snapshot.kind)) &&
      matchesStored(snapshot, filters.stored) &&
      new Date(snapshot.createdAt).getTime() >= cutoff,
  );
}
