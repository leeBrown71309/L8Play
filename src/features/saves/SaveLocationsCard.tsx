import { useLingui } from "@lingui/react/macro";
import { useState } from "react";

import { StatusBadge } from "@/components/ds/status-badge";
import { notify } from "@/components/feedback/toast-store";
import { Banner } from "@/components/ds/page-blocks";
import { Button } from "@/components/ui/button";
import { IconButton } from "@/components/ui/icon-button";
import { FindSavesDialog } from "@/features/saves/FindSavesDialog";
import { useSnapshotLabels } from "@/features/saves/snapshot-labels";
import { useFolderPicker } from "@/features/saves/use-folder-picker";
import { useAddSaveFolder, useDetectSaves, useRemoveSaveLocation, useSaveLocations } from "@/features/saves/use-saves";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import { cn } from "@/lib/cn";

interface SaveLocationsCardProps {
  game: GameSummary;
  unknownToDetector: boolean;
}

/** Where the game keeps its saves: detected by Ludusavi or added by hand, with re-detect and add-folder. */
export function SaveLocationsCard({ game, unknownToDetector }: SaveLocationsCardProps) {
  const { t } = useLingui();
  const labels = useSnapshotLabels();
  const locations = useSaveLocations(game.id);
  const detect = useDetectSaves();
  const add = useAddSaveFolder();
  const remove = useRemoveSaveLocation();
  const pickFolder = useFolderPicker();
  const [finding, setFinding] = useState(false);

  const runDetection = () =>
    detect.mutate(game.id, {
      onSuccess: (outcome) => {
        const count = outcome.locations.length;
        if (outcome.status === "found") {
          notify({
            tone: "success",
            title: count === 1 ? t`1 save location found` : t`${count} save locations found`,
            body: t`Matched “${outcome.manifestTitle}” in the save database.`,
          });
        } else {
          notify({
            tone: "warning",
            title: t`${game.title} is not in the save database`,
            body: t`Add its save folder yourself to protect it.`,
          });
        }
      },
      onError: (error) => notify({ tone: "danger", title: t`Save detection failed`, body: labels.error(error) }),
    });

  const addFolder = async () => {
    const folder = await pickFolder(t`Choose the folder where ${game.title} keeps its saves`);
    if (!folder) return;
    add.mutate(
      { gameId: game.id, folder },
      {
        onSuccess: () => notify({ tone: "success", title: t`Save folder added`, body: folder }),
        onError: (error) => notify({ tone: "danger", title: t`This folder was not added`, body: labels.error(error) }),
      },
    );
  };

  const items = locations.data ?? [];
  return (
    <section className="gd-card">
      <h3>
        {t`Save locations`}
        <span className="flex gap-1">
          <Button size="sm" variant="ghost" icon="scan" loading={detect.isPending} onClick={runDetection}>
            {t`Detect again`}
          </Button>
          <Button
            size="sm"
            variant={unknownToDetector ? "primary" : "ghost"}
            icon="search"
            onClick={() => setFinding(true)}
          >
            {t`Find saves`}
          </Button>
          <Button size="sm" variant="ghost" icon="plus" disabled={add.isPending} onClick={() => void addFolder()}>
            {t`Add`}
          </Button>
        </span>
      </h3>

      {items.length === 0 && !locations.isPending ? (
        <Banner tone="warning" title={t`No save location yet`}>
          {unknownToDetector
            ? t`The save database doesn’t know this game. Add its save folder so L8Play can protect it.`
            : t`Run detection or add the folder yourself.`}
        </Banner>
      ) : null}

      {items.map((location) => (
        <div key={location.id} className="gd-loc">
          <div className="gd-loc-h">
            {location.kind === "file" ? t`Save file` : t`Game save folder`}
            <span className="flex items-center gap-1">
              {location.source === "user" ? (
                <StatusBadge status="info" size="sm">
                  {t`Added by you`}
                </StatusBadge>
              ) : (
                <StatusBadge status="verified" size="sm">
                  {t`Detected`}
                </StatusBadge>
              )}
              <IconButton
                icon="x"
                label={t`Stop backing up this location`}
                size="sm"
                disabled={remove.isPending}
                onClick={() =>
                  remove.mutate(
                    { gameId: game.id, locationId: location.id },
                    {
                      onError: (error) =>
                        notify({ tone: "danger", title: t`The location was not removed`, body: labels.error(error) }),
                    },
                  )
                }
              />
            </span>
          </div>
          <div
            className={cn("gd-loc-p", !location.exists && "is-missing")}
            title={location.exists ? location.path : t`This path doesn’t exist right now`}
          >
            {location.path}
          </div>
        </div>
      ))}
      <FindSavesDialog game={game} open={finding} onOpenChange={setFinding} />
    </section>
  );
}
