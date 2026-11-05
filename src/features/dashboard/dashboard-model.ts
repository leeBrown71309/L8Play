import type { ConflictInfo } from "@/lib/bindings/ConflictInfo";
import type { GameSaveStatus } from "@/lib/bindings/GameSaveStatus";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import type { SyncOperationInfo } from "@/lib/bindings/SyncOperationInfo";

export type AttentionKind = "conflict" | "uploading" | "queued" | "unprotected" | "no-snapshot";

export interface AttentionItem {
  kind: AttentionKind;
  game: GameSummary;
  /** Snapshots waiting or uploading for this game (upload kinds). */
  operations: number;
  bytesDone: number;
  bytesTotal: number;
  /** Device that holds the other save (conflicts). */
  remoteDevice: string | null;
  snapshotName: string | null;
}

interface AttentionInput {
  games: readonly GameSummary[];
  statuses: readonly GameSaveStatus[];
  conflicts: readonly ConflictInfo[];
  operations: readonly SyncOperationInfo[];
  limit: number;
}

const base = (kind: AttentionKind, game: GameSummary): AttentionItem => ({
  kind,
  game,
  operations: 0,
  bytesDone: 0,
  bytesTotal: 0,
  remoteDevice: null,
  snapshotName: null,
});

/**
 * What needs the user, most urgent first: sync conflicts, uploads in progress or waiting, then
 * installed games whose save is not protected (most played first).
 */
export function buildAttention({ games, statuses, conflicts, operations, limit }: AttentionInput): AttentionItem[] {
  const byId = new Map(games.map((game) => [game.id, game]));
  const items: AttentionItem[] = [];

  for (const conflict of conflicts) {
    const game = byId.get(conflict.gameId);
    if (game) items.push({ ...base("conflict", game), remoteDevice: conflict.remote.deviceName });
  }

  const uploads = new Map<number, AttentionItem>();
  for (const operation of operations) {
    if (operation.direction !== "upload" || (operation.status !== "running" && operation.status !== "queued")) continue;
    const game = games.find((candidate) => candidate.title === operation.gameTitle);
    if (!game || items.some((item) => item.kind === "conflict" && item.game.id === game.id)) continue;
    const item = uploads.get(game.id) ?? base("queued", game);
    item.operations += 1;
    item.bytesDone += operation.bytesDone;
    item.bytesTotal += operation.bytesTotal;
    item.snapshotName ??= operation.snapshotName;
    if (operation.status === "running") item.kind = "uploading";
    uploads.set(game.id, item);
  }
  items.push(...uploads.values());

  const statusOf = new Map(statuses.map((status) => [status.gameId, status]));
  const exposed = games
    .filter((game) => game.installed)
    .filter((game) => !items.some((item) => item.game.id === game.id))
    .map((game) => ({ game, status: statusOf.get(game.id) }))
    .filter(({ status }) => status !== undefined && status.snapshotCount === 0)
    .sort((a, b) => b.game.playtimeSeconds - a.game.playtimeSeconds);
  for (const { game, status } of exposed) {
    items.push(base(status && status.locationCount > 0 ? "no-snapshot" : "unprotected", game));
  }

  return items.slice(0, limit);
}

/** Share of a transfer that is done, 0–100. */
export function transferPercent(bytesDone: number, bytesTotal: number): number {
  return bytesTotal > 0 ? Math.min(100, Math.round((bytesDone / bytesTotal) * 100)) : 0;
}

/** Uploads to show in "Backing up now": running first, then waiting. */
export function activeUploads(operations: readonly SyncOperationInfo[], limit: number): SyncOperationInfo[] {
  const rank = (operation: SyncOperationInfo) => (operation.status === "running" ? 0 : 1);
  return operations
    .filter((operation) => operation.status === "running" || operation.status === "queued")
    .sort((a, b) => rank(a) - rank(b))
    .slice(0, limit);
}
