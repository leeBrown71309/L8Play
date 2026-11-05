import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import type { SnapshotStorage } from "@/lib/bindings/SnapshotStorage";
import { queryKeys } from "@/lib/query-keys";
import {
  addSaveFolder,
  createSnapshot,
  deleteSnapshot,
  detectSaves,
  getCurrentSave,
  getSaveStatuses,
  getSnapshotFolder,
  listSaveLocations,
  listSnapshots,
  removeSaveLocation,
  restoreSnapshot,
  revealSnapshot,
  setSnapshotPinned,
  verifySnapshot,
} from "@/lib/tauri/commands";

export function useSaveLocations(gameId: number) {
  return useQuery({ queryKey: queryKeys.saveLocations(gameId), queryFn: () => listSaveLocations(gameId) });
}

export function useSnapshots(gameId: number | null) {
  return useQuery({ queryKey: queryKeys.snapshots(gameId), queryFn: () => listSnapshots(gameId) });
}

export function useSaveStatuses() {
  return useQuery({ queryKey: queryKeys.saveStatuses, queryFn: getSaveStatuses });
}

/** Hashes the current save on demand, so it is refreshed whenever the Saves tab opens. */
export function useCurrentSave(gameId: number) {
  return useQuery({ queryKey: queryKeys.currentSave(gameId), queryFn: () => getCurrentSave(gameId), staleTime: 0 });
}

export function useSnapshotFolder() {
  return useQuery({ queryKey: queryKeys.snapshotFolder, queryFn: getSnapshotFolder });
}

/** Every save mutation refreshes all save queries (locations, snapshots, statuses, current save). */
function useSaveMutation<TArgs, TResult>(mutationFn: (args: TArgs) => Promise<TResult>) {
  const client = useQueryClient();
  return useMutation({ mutationFn, onSettled: () => client.invalidateQueries({ queryKey: queryKeys.saves }) });
}

export const useDetectSaves = () => useSaveMutation((gameId: number) => detectSaves(gameId));
export const useAddSaveFolder = () =>
  useSaveMutation(({ gameId, folder }: { gameId: number; folder: string }) => addSaveFolder(gameId, folder));
export const useRemoveSaveLocation = () =>
  useSaveMutation(({ gameId, locationId }: { gameId: number; locationId: number }) =>
    removeSaveLocation(gameId, locationId),
  );
export const useCreateSnapshot = () =>
  useSaveMutation(
    ({
      gameId,
      name,
      description,
      pinned,
      storage,
    }: {
      gameId: number;
      name: string;
      description: string | null;
      pinned: boolean;
      storage: SnapshotStorage;
    }) => createSnapshot(gameId, name, description, pinned, storage),
  );
export const useRestoreSnapshot = () => useSaveMutation((snapshotId: string) => restoreSnapshot(snapshotId));
export const useDeleteSnapshot = () =>
  useSaveMutation(({ snapshotId, alsoFromDrive }: { snapshotId: string; alsoFromDrive: boolean }) =>
    deleteSnapshot(snapshotId, alsoFromDrive),
  );
export const useVerifySnapshot = () => useSaveMutation((snapshotId: string) => verifySnapshot(snapshotId));
export const useSetSnapshotPinned = () =>
  useSaveMutation(({ snapshotId, pinned }: { snapshotId: string; pinned: boolean }) =>
    setSnapshotPinned(snapshotId, pinned),
  );
export const useRevealSnapshot = () =>
  useMutation({ mutationFn: (snapshotId: string | null) => revealSnapshot(snapshotId) });
