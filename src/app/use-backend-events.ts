import { useLingui } from "@lingui/react/macro";
import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";

import { notify } from "@/components/feedback/toast-store";
import { useArtworkLoading } from "@/features/library/artwork-loading-store";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import { formatPlaytime } from "@/lib/format";
import { queryKeys } from "@/lib/query-keys";
import { getArtworkPending } from "@/lib/tauri/commands";
import { EVENTS, onBackendEvent } from "@/lib/tauri/events";

/** Keeps cached data in sync with backend events and reports finished play sessions. */
export function useBackendEvents(): void {
  const client = useQueryClient();
  const { t } = useLingui();

  useEffect(() => {
    const titleOf = (gameId: number): string =>
      client.getQueryData<GameSummary[]>(queryKeys.games)?.find((game) => game.id === gameId)?.title ?? "";
    // The startup lookup may start before this window listens, so the backend's own list is asked for too.
    const syncPendingArtwork = () => {
      getArtworkPending()
        .then((ids) => useArtworkLoading.getState().syncPending(ids))
        .catch((error: unknown) => console.warn("Could not read the pending artwork list.", error));
    };
    syncPendingArtwork();
    const pendingPoll = window.setInterval(() => {
      if (useArtworkLoading.getState().pending.size > 0) syncPendingArtwork();
    }, 4000);
    const subscriptions = [
      onBackendEvent(EVENTS.libraryChanged, () => {
        void client.invalidateQueries({ queryKey: queryKeys.games });
        syncPendingArtwork();
      }),
      onBackendEvent(EVENTS.artworkFetched, (gameId) => {
        useArtworkLoading.getState().finish(gameId);
        void client.invalidateQueries({ queryKey: queryKeys.games });
      }),
      onBackendEvent(EVENTS.savesChanged, () => {
        void client.invalidateQueries({ queryKey: queryKeys.saves });
      }),
      onBackendEvent(EVENTS.cloudChanged, (event) => {
        void client.invalidateQueries({ queryKey: queryKeys.cloud });
        void client.invalidateQueries({ queryKey: queryKeys.saves });
        if (event.newConflicts > 0) {
          notify({
            tone: "warning",
            title: t`Two different saves found`,
            body: t`A game was played on two devices. Open Cloud Storage to choose which save to keep.`,
          });
        }
      }),
      onBackendEvent(EVENTS.autoSnapshot, (event) => {
        const title = titleOf(event.gameId);
        // "unchanged" and "nothing" are silent: the save was already protected or there is nothing to copy.
        if (event.status === "created") {
          notify({
            tone: "success",
            title: event.kind === "auto_launch" ? t`Save backed up before launch` : t`Save backed up`,
            body: title,
          });
        } else if (event.status === "busy") {
          notify({
            tone: "warning",
            title: t`Automatic snapshot skipped`,
            body: t`${title}: the save files were still in use. Create a snapshot from the game page.`,
          });
        } else if (event.status === "failed") {
          notify({
            tone: "danger",
            title: t`Automatic snapshot failed`,
            body: t`${title}: check the log file for details.`,
          });
        }
      }),
      onBackendEvent(EVENTS.gameSession, (event) => {
        client.setQueryData<number[]>(queryKeys.runningGames, (current = []) =>
          event.running ? [...new Set([...current, event.gameId])] : current.filter((id) => id !== event.gameId),
        );
        if (!event.running && event.durationSeconds !== null && event.durationSeconds >= 60) {
          const title = titleOf(event.gameId);
          notify({
            tone: "neutral",
            title: t`Session ended`,
            body: t`${title} · ${formatPlaytime(event.durationSeconds)} played`,
          });
        }
      }),
    ];
    return () => {
      window.clearInterval(pendingPoll);
      for (const subscription of subscriptions) void subscription.then((unlisten) => unlisten());
    };
  }, [client, t]);
}
