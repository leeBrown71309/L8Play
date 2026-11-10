import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";

import { useArtworkLoading } from "@/features/library/artwork-loading-store";
import { useImportStore } from "@/features/saves-import/import-store";
import type { ImportGameRequest } from "@/lib/bindings/ImportGameRequest";
import type { PlannedLayout } from "@/lib/bindings/PlannedLayout";
import type { ScanMode } from "@/lib/bindings/ScanMode";
import { queryKeys } from "@/lib/query-keys";
import {
  importBackupGames,
  replanBackupGame,
  scanBackupFolder,
  scanPcForLeftoverSaves,
} from "@/lib/tauri/commands";
import { EVENTS, onBackendEvent } from "@/lib/tauri/events";

export function useScanBackupFolder() {
  return useMutation({
    mutationFn: ({ folder, mode }: { folder: string; mode: ScanMode | null }) => scanBackupFolder(folder, mode),
  });
}

/** Looks for the leftover saves of uninstalled games on this PC (no folder to choose). */
export function useScanPcForSaves() {
  return useMutation({ mutationFn: () => scanPcForLeftoverSaves() });
}

export function useReplanBackupGame() {
  return useMutation({
    mutationFn: (input: { folder: string; key: string; fromCollection: string | null; layout: PlannedLayout }) =>
      replanBackupGame(input.folder, input.key, input.fromCollection, input.layout),
  });
}

export function useImportBackupGames() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (games: ImportGameRequest[]) => importBackupGames(games),
    // New games get their artwork looked up in the background: their cards shimmer until it reports back.
    onSuccess: (report) => {
      const created = report.games.flatMap((game) => (game.created && game.gameId !== null ? [game.gameId] : []));
      useArtworkLoading.getState().markPending(created);
    },
    onSettled: () => {
      void client.invalidateQueries({ queryKey: queryKeys.games });
      void client.invalidateQueries({ queryKey: queryKeys.saves });
    },
  });
}

/** Feeds the progress bar while games are being imported. */
export function useImportProgress(): void {
  useEffect(() => {
    let stop: (() => void) | undefined;
    let cancelled = false;
    void onBackendEvent(EVENTS.importProgress, (progress) => useImportStore.getState().progress(progress)).then(
      (unlisten) => {
        if (cancelled) unlisten();
        else stop = unlisten;
      },
    );
    return () => {
      cancelled = true;
      stop?.();
    };
  }, []);
}
