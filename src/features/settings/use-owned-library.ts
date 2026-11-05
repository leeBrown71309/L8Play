import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";

import type { OwnedLibraryStatus } from "@/lib/bindings/OwnedLibraryStatus";
import { queryKeys } from "@/lib/query-keys";
import { clearSteamKey, getOwnedLibraryStatus, getSteamKeyStatus, saveSteamKey } from "@/lib/tauri/commands";
import { EVENTS, onBackendEvent } from "@/lib/tauri/events";

/**
 * What the last "games I own but have not installed" lookup found. Every scan ends with a library
 * change event, which refreshes it.
 */
export function useOwnedLibraryStatus(): OwnedLibraryStatus {
  const client = useQueryClient();
  const { data } = useQuery({ queryKey: queryKeys.ownedLibraryStatus, queryFn: getOwnedLibraryStatus });
  useEffect(() => {
    const subscription = onBackendEvent(EVENTS.libraryChanged, () => {
      void client.invalidateQueries({ queryKey: queryKeys.ownedLibraryStatus });
    });
    return () => {
      void subscription.then((unlisten) => unlisten());
    };
  }, [client]);
  return data ?? "disabled";
}

/** Whether a Steam Web API key is saved. The key itself never comes back from the backend. */
export function useSteamKeyStatus() {
  return useQuery({ queryKey: queryKeys.steamKey, queryFn: getSteamKeyStatus });
}

function useSteamKeyMutation<TArgs>(mutationFn: (args: TArgs) => Promise<null>) {
  const client = useQueryClient();
  return useMutation({
    mutationFn,
    onSettled: () => {
      void client.invalidateQueries({ queryKey: queryKeys.steamKey });
      void client.invalidateQueries({ queryKey: queryKeys.ownedLibraryStatus });
    },
  });
}

export const useSaveSteamKey = () => useSteamKeyMutation((key: string) => saveSteamKey(key));
export const useClearSteamKey = () => useSteamKeyMutation(() => clearSteamKey());
