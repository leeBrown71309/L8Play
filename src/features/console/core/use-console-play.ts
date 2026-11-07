import { useLingui } from "@lingui/react/macro";

import { minimizeForGame, useConsoleGameWindow } from "@/features/console/core/game-window";
import { consoleToast } from "@/features/console/core/toast-store";
import { rumble } from "@/features/console/input";
import { consoleSound } from "@/features/console/sound";
import { useLaunchGame, useToggleFavorite } from "@/features/library/use-library";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import { getRunningGames } from "@/lib/tauri/commands";
import { AppError } from "@/lib/tauri/errors";

/** Play / install and favourite, answered with console toasts, a launch sound and a short rumble. */
export function useConsoleGameActions() {
  const { t } = useLingui();
  const launch = useLaunchGame();
  const favorite = useToggleFavorite();

  return {
    play(game: GameSummary) {
      // Nothing to launch for a saves-only game.
      if (game.archived) return;
      consoleSound.play("launch");
      rumble({ strong: 0.5, weak: 0.5, durationMs: 220 });
      launch.mutate(game.id, {
        onSuccess: () => {
          consoleToast({
            tone: "success",
            title: game.installed ? t`Starting ${game.title}` : t`Opening the installer for ${game.title}`,
          });
          // An installer page is not a play session: only a real game makes the console step aside.
          if (game.installed) {
            useConsoleGameWindow.getState().handOver(game.id);
            consoleSound.stopAmbient();
            void minimizeForGame();
            // The launch can take long (the Windows administrator prompt waits for an answer), so the game
            // may already have started and its "started" event gone by: ask the backend what is running.
            getRunningGames()
              .then((ids) => {
                if (ids.includes(game.id)) useConsoleGameWindow.getState().onSession(game.id, true);
              })
              .catch((error: unknown) => console.warn("Could not read the running games.", error));
          }
        },
        onError: (error) => {
          consoleSound.play("error");
          consoleToast({
            tone: "danger",
            title: t`${game.title} could not be started`,
            body: error instanceof AppError && error.code === "validation" ? error.message : undefined,
          });
        },
      });
    },
    toggleFavorite(game: GameSummary) {
      consoleSound.play("toggle");
      favorite.mutate(
        { gameId: game.id, favorite: !game.favorite },
        { onError: () => consoleToast({ tone: "danger", title: t`The favourite could not be saved` }) },
      );
    },
    isLaunching: (gameId: number) => launch.isPending && launch.variables === gameId,
  };
}
