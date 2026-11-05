import { useLingui } from "@lingui/react/macro";
import { open } from "@tauri-apps/plugin-dialog";
import { useNavigate } from "react-router";

import { notify } from "@/components/feedback/toast-store";
import { useAddGameStore } from "@/features/library/add-game-store";
import { useAddStandaloneGame, useScanLibraries } from "@/features/library/use-library";
import type { ScanReport } from "@/lib/bindings/ScanReport";
import { AppError } from "@/lib/tauri/errors";
import { isTauri } from "@/lib/tauri/runtime";

/** "Scan libraries" with a result toast that says what changed. */
export function useScanFlow() {
  const { t } = useLingui();
  const scan = useScanLibraries();

  const summarize = (report: ScanReport) => {
    const failed = report.platforms.filter((platform) => platform.error !== null);
    if (failed.length > 0) {
      return {
        tone: "warning" as const,
        title: t`Some libraries could not be read`,
        body: failed.map((platform) => platform.error).join(" · "),
      };
    }
    const { added, uninstalled, total } = report;
    const body =
      added === 0 && uninstalled === 0
        ? t`No changes — ${total} games in your library.`
        : t`${added} new, ${uninstalled} no longer installed — ${total} games in total.`;
    return { tone: "success" as const, title: t`Libraries scanned`, body };
  };

  return {
    start: () =>
      scan.mutate(undefined, {
        onSuccess: (report) => notify(summarize(report)),
        onError: () =>
          notify({
            tone: "danger",
            title: t`The scan failed`,
            body: t`Check the log file for details, then try again.`,
          }),
      }),
    isScanning: scan.isPending,
  };
}

/** "Add a game": opens the chooser (scan a folder, or pick one executable). */
export function useAddGameFlow() {
  const openChooser = useAddGameStore((state) => state.openChooser);
  return { start: openChooser, isAdding: false };
}

/** Native file picker for one .exe, then opens the new game's page. */
export function useAddExecutableFlow() {
  const { t } = useLingui();
  const navigate = useNavigate();
  const add = useAddStandaloneGame();

  return {
    start: async () => {
      if (!isTauri()) {
        notify({ tone: "neutral", title: t`Adding games needs the desktop app` });
        return;
      }
      let picked: string | null;
      try {
        picked = await open({
          multiple: false,
          directory: false,
          title: t`Choose the game's executable`,
          filters: [{ name: t`Programs`, extensions: ["exe"] }],
        });
      } catch (error) {
        console.error("File picker failed.", error);
        notify({ tone: "danger", title: t`The file picker could not be opened` });
        return;
      }
      if (!picked) return;
      add.mutate(picked, {
        onSuccess: (game) => {
          notify({ tone: "success", title: t`${game.title} added to your library` });
          void navigate(`/games/${game.id}`);
        },
        onError: (error) =>
          notify({
            tone: "danger",
            title: t`This game could not be added`,
            body: error instanceof AppError && error.code === "validation" ? error.message : undefined,
          }),
      });
    },
    isAdding: add.isPending,
  };
}
