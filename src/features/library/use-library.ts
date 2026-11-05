import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { useArtworkLoading } from "@/features/library/artwork-loading-store";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import type { NewGameInput } from "@/lib/bindings/NewGameInput";
import { queryKeys } from "@/lib/query-keys";
import {
  addGames,
  addStandaloneGame,
  getGame,
  getRunningGames,
  launchGame,
  listGames,
  openInstallFolder,
  scanGameFolder,
  scanLibraries,
  setFavorite,
} from "@/lib/tauri/commands";

export function useGames() {
  return useQuery({ queryKey: queryKeys.games, queryFn: listGames });
}

export function useGame(gameId: number) {
  return useQuery({
    queryKey: queryKeys.game(gameId),
    queryFn: () => getGame(gameId),
    enabled: Number.isFinite(gameId),
  });
}

/** Ids of games currently running (kept fresh by `game://session` events). */
export function useRunningGames(): Set<number> {
  const { data } = useQuery({ queryKey: queryKeys.runningGames, queryFn: getRunningGames });
  return new Set(data ?? []);
}

export function useScanLibraries() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: scanLibraries,
    onSettled: () => client.invalidateQueries({ queryKey: queryKeys.games }),
  });
}

export function useToggleFavorite() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ gameId, favorite }: { gameId: number; favorite: boolean }) => setFavorite(gameId, favorite),
    onMutate: async ({ gameId, favorite }) => {
      await client.cancelQueries({ queryKey: queryKeys.games });
      const previous = client.getQueryData<GameSummary[]>(queryKeys.games);
      client.setQueryData<GameSummary[]>(queryKeys.games, (games) =>
        games?.map((game) => (game.id === gameId ? { ...game, favorite } : game)),
      );
      return { previous };
    },
    onError: (_error, _vars, context) => client.setQueryData(queryKeys.games, context?.previous),
    onSettled: () => client.invalidateQueries({ queryKey: queryKeys.games }),
  });
}

export function useLaunchGame() {
  return useMutation({ mutationFn: (gameId: number) => launchGame(gameId) });
}

export function useOpenInstallFolder() {
  return useMutation({ mutationFn: (gameId: number) => openInstallFolder(gameId) });
}

export function useScanGameFolder() {
  return useMutation({ mutationFn: (folder: string) => scanGameFolder(folder) });
}

export function useAddGames() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (games: NewGameInput[]) => addGames(games),
    // The backend now looks artwork up in the background: the new cards shimmer until it reports back.
    onSuccess: (report) => useArtworkLoading.getState().markPending(report.added.map((game) => game.id)),
    onSettled: () => client.invalidateQueries({ queryKey: queryKeys.games }),
  });
}

export function useAddStandaloneGame() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (executablePath: string) => addStandaloneGame(executablePath),
    onSuccess: (game) => useArtworkLoading.getState().markPending([game.id]),
    onSettled: () => client.invalidateQueries({ queryKey: queryKeys.games }),
  });
}
