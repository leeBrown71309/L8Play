import { useLingui } from "@lingui/react/macro";

import { notify } from "@/components/feedback/toast-store";
import { useLaunchGame, useOpenInstallFolder, useToggleFavorite } from "@/features/library/use-library";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import { AppError } from "@/lib/tauri/errors";

/** Play / favourite / open-folder actions with user-facing result toasts. */
export function useGameActions() {
  const { t } = useLingui();
  const launch = useLaunchGame();
  const favorite = useToggleFavorite();
  const openFolder = useOpenInstallFolder();

  const describe = (error: unknown) =>
    error instanceof AppError && error.code === "validation" ? error.message : undefined;

  return {
    play(game: GameSummary) {
      launch.mutate(game.id, {
        onSuccess: () => {
          const via = {
            steam: t`through Steam`,
            epic: t`through the Epic Games Launcher`,
            xbox: t`through the Xbox app`,
            gog: t`through GOG Galaxy`,
            standalone: t`directly`,
          }[game.platform];
          notify({
            tone: "neutral",
            title: game.installed ? t`Starting ${game.title}` : t`Opening the installer for ${game.title}`,
            body: game.installed
              ? t`Launching ${via}. Play time is recorded automatically.`
              : t`The installation continues ${via}.`,
          });
        },
        onError: (error) =>
          notify({ tone: "danger", title: t`${game.title} could not be started`, body: describe(error) }),
      });
    },
    toggleFavorite(game: GameSummary) {
      favorite.mutate(
        { gameId: game.id, favorite: !game.favorite },
        { onError: () => notify({ tone: "danger", title: t`The favourite could not be saved` }) },
      );
    },
    openFolder(game: GameSummary) {
      openFolder.mutate(game.id, {
        onError: (error) =>
          notify({ tone: "warning", title: t`The install folder could not be opened`, body: describe(error) }),
      });
    },
    isLaunching: (gameId: number) => launch.isPending && launch.variables === gameId,
  };
}
