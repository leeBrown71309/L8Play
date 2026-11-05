import { useLingui } from "@lingui/react/macro";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import type { ConflictResolution } from "@/lib/bindings/ConflictResolution";
import { queryKeys } from "@/lib/query-keys";
import {
  clearGoogleClient,
  connectGoogleDrive,
  disconnectGoogleDrive,
  downloadSnapshot,
  getCloudQuota,
  getCloudStatus,
  listConflicts,
  listSyncOperations,
  resolveConflict,
  retrySync,
  saveGoogleClient,
  syncCloudNow,
} from "@/lib/tauri/commands";
import { AppError } from "@/lib/tauri/errors";

export function useCloudStatus() {
  return useQuery({ queryKey: queryKeys.cloudStatus, queryFn: getCloudStatus });
}

/** Storage figures are a network call: only asked while a connected account is shown. */
export function useCloudQuota(enabled: boolean) {
  return useQuery({
    queryKey: queryKeys.cloudQuota,
    queryFn: getCloudQuota,
    enabled,
    staleTime: 5 * 60 * 1000,
    retry: false,
  });
}

export function useSyncOperations(enabled: boolean) {
  return useQuery({ queryKey: queryKeys.cloudOperations, queryFn: listSyncOperations, enabled });
}

export function useConflicts(enabled: boolean) {
  return useQuery({ queryKey: queryKeys.cloudConflicts, queryFn: listConflicts, enabled });
}

/** Cloud and save lists depend on each other (badges, cloud-only snapshots): refresh both. */
function useCloudMutation<TArgs, TResult>(mutationFn: (args: TArgs) => Promise<TResult>) {
  const client = useQueryClient();
  return useMutation({
    mutationFn,
    onSettled: () => {
      void client.invalidateQueries({ queryKey: queryKeys.cloud });
      void client.invalidateQueries({ queryKey: queryKeys.saves });
    },
  });
}

export const useSaveGoogleClient = () =>
  useCloudMutation(({ clientId, clientSecret }: { clientId: string; clientSecret: string }) =>
    saveGoogleClient(clientId, clientSecret),
  );
export const useClearGoogleClient = () => useCloudMutation(() => clearGoogleClient());
export const useConnectGoogleDrive = () => useCloudMutation(() => connectGoogleDrive());
export const useDisconnectGoogleDrive = () => useCloudMutation(() => disconnectGoogleDrive());
export const useSyncNow = () => useCloudMutation(() => syncCloudNow());
export const useRetrySync = () => useCloudMutation(() => retrySync());
export const useResolveConflict = () =>
  useCloudMutation(({ conflictId, resolution }: { conflictId: number; resolution: ConflictResolution }) =>
    resolveConflict(conflictId, resolution),
  );
export const useDownloadSnapshot = () => useCloudMutation((snapshotId: string) => downloadSnapshot(snapshotId));

/** Translated reasons for cloud failures. */
export function useCloudErrorText() {
  const { t } = useLingui();
  return (failure: unknown): string => {
    if (!(failure instanceof AppError)) return t`Something went wrong. Check the log file for details.`;
    switch (failure.code) {
      case "auth":
        return t`Google refused the sign-in. Check the OAuth client, or sign in again.`;
      case "busy":
        return t`Google or the game is busy right now. Try again in a moment.`;
      case "external":
        return t`Google could not be reached. Check your connection and try again.`;
      case "integrity":
        return t`The downloaded copy did not match its checksum and was discarded.`;
      case "validation":
      case "not_found":
        return failure.message;
      default:
        return t`Something went wrong. Check the log file for details.`;
    }
  };
}
