import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";

/** Where a snapshot's copies are, as words: "This PC and Google Drive". */
export function storedIn(snapshot: SnapshotSummary, labels: { pc: string; drive: string; both: string }): string {
  const onPc = snapshot.archivePath !== null && snapshot.status !== "missing_local";
  const inDrive = snapshot.cloud === "synced";
  if (onPc && inDrive) return labels.both;
  return onPc ? labels.pc : inDrive ? labels.drive : labels.pc;
}

/** Newest first. */
export function newestFirst(snapshots: readonly SnapshotSummary[]): SnapshotSummary[] {
  return [...snapshots].sort((a, b) => b.createdAt.localeCompare(a.createdAt));
}
