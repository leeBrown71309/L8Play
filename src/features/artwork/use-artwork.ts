import { useLingui } from "@lingui/react/macro";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import type { ArtworkCandidate } from "@/lib/bindings/ArtworkCandidate";
import type { ArtworkKind } from "@/lib/bindings/ArtworkKind";
import { queryKeys } from "@/lib/query-keys";
import {
  applyArtwork,
  clearArtworkKey,
  fetchMissingArtwork,
  findArtwork,
  getArtworkKeyStatus,
  getArtworkSelections,
  importArtwork,
  resetArtwork,
  saveArtworkKey,
  updateGameMetadata,
  verifyArtworkKey,
} from "@/lib/tauri/commands";
import { AppError } from "@/lib/tauri/errors";

export function useArtworkKeyStatus() {
  return useQuery({ queryKey: queryKeys.artworkKey, queryFn: getArtworkKeyStatus });
}

export function useArtworkSelections(gameId: number) {
  return useQuery({ queryKey: queryKeys.artworkSelections(gameId), queryFn: () => getArtworkSelections(gameId) });
}

/** Provider results are cached for ten minutes so flipping between kinds does not re-ask the service. */
export function useArtworkCandidates(gameId: number, kind: ArtworkKind, term: string, enabled: boolean) {
  return useQuery({
    queryKey: queryKeys.artworkCandidates(gameId, kind, term),
    queryFn: () => findArtwork(gameId, kind, term.trim() === "" ? null : term.trim()),
    enabled,
    staleTime: 10 * 60 * 1000,
    retry: false,
  });
}

/** Library lists, selections and key status all depend on artwork. */
function useArtworkMutation<TArgs, TResult>(mutationFn: (args: TArgs) => Promise<TResult>) {
  const client = useQueryClient();
  return useMutation({
    mutationFn,
    onSettled: () => {
      // Candidates are deliberately not invalidated: the picker must not reload (and flash) after a choice.
      void client.invalidateQueries({ queryKey: ["artwork", "selections"] });
      void client.invalidateQueries({ queryKey: queryKeys.artworkKey });
      void client.invalidateQueries({ queryKey: queryKeys.games });
    },
  });
}

export const useSaveArtworkKey = () => useArtworkMutation((key: string) => saveArtworkKey(key));
export const useClearArtworkKey = () => useArtworkMutation(() => clearArtworkKey());
export const useVerifyArtworkKey = () => useMutation({ mutationFn: verifyArtworkKey });
export const useFetchMissingArtwork = () => useArtworkMutation(() => fetchMissingArtwork());
export const useApplyArtwork = () =>
  useArtworkMutation(({ gameId, candidate }: { gameId: number; candidate: ArtworkCandidate }) =>
    applyArtwork(gameId, candidate),
  );
export const useImportArtwork = () =>
  useArtworkMutation(({ gameId, kind, path }: { gameId: number; kind: ArtworkKind; path: string }) =>
    importArtwork(gameId, kind, path),
  );
export const useResetArtwork = () =>
  useArtworkMutation(({ gameId, kind }: { gameId: number; kind: ArtworkKind }) => resetArtwork(gameId, kind));
export const useUpdateGameMetadata = () =>
  useArtworkMutation((args: { gameId: number; title: string; developer: string | null; releaseYear: number | null }) =>
    updateGameMetadata(args.gameId, args.title, args.developer, args.releaseYear),
  );

/** Translated, user-facing reasons for artwork and metadata failures. */
export function useArtworkErrorText() {
  const { t } = useLingui();
  return (failure: unknown): string => {
    if (!(failure instanceof AppError)) return t`Something went wrong. Check the log file for details.`;
    switch (failure.code) {
      case "auth":
        return t`SteamGridDB refused the API key. Check it in Settings › Artwork.`;
      case "busy":
        return t`SteamGridDB is limiting requests right now. Try again in a minute.`;
      case "external":
        return t`SteamGridDB could not be reached. Check your connection and try again.`;
      case "validation":
        return failure.message;
      default:
        return t`Something went wrong. Check the log file for details.`;
    }
  };
}
