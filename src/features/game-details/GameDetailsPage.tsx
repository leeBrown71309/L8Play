import { useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { Link, useParams } from "react-router";

import { ROUTES } from "@/app/routes";
import { Tabs } from "@/components/ds/form-fields";
import { EmptyState } from "@/components/feedback/EmptyState";
import { Skeleton } from "@/components/feedback/Skeleton";
import { WarningIcon } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { ArtworkDialog } from "@/features/artwork/ArtworkDialog";
import { GameHero } from "@/features/game-details/GameHero";
import { GameOverview } from "@/features/game-details/GameOverview";
import { PropertiesPanel } from "@/features/game-details/PropertiesPanel";
import "@/features/game-details/game-details.css";
import { useGame, useRunningGames } from "@/features/library/use-library";
import { CreateSnapshotDialog } from "@/features/saves/CreateSnapshotDialog";
import { DeleteSnapshotDialog } from "@/features/saves/DeleteSnapshotDialog";
import { GameSavesPanel } from "@/features/saves/GameSavesPanel";
import { RestoreSnapshotDialog } from "@/features/saves/RestoreSnapshotDialog";
import { summarizeCopies } from "@/features/saves/snapshot-summary";
import { useSaveStatuses, useSnapshots } from "@/features/saves/use-saves";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";

type DetailsTab = "overview" | "saves" | "artwork" | "properties";

/** Game details: the hero with its tabs, then Overview, Saves or Properties; Artwork opens as a dialog. */
export function GameDetailsPage() {
  const { t } = useLingui();
  const gameId = Number(useParams().gameId);
  const { data: detail, isPending, isError } = useGame(gameId);
  const running = useRunningGames().has(gameId);
  const snapshots = useSnapshots(gameId);
  const status = useSaveStatuses().data?.find((entry) => entry.gameId === gameId);
  const [tab, setTab] = useState<DetailsTab>("overview");
  const [creating, setCreating] = useState(false);
  const [restoring, setRestoring] = useState<SnapshotSummary | null>(null);
  const [deleting, setDeleting] = useState<SnapshotSummary | null>(null);

  if (isPending) return <Skeleton className="h-[360px] rounded-lg" />;
  if (isError) {
    return (
      <div className="rounded-lg bg-surface-200">
        <EmptyState
          icon={<WarningIcon size={30} />}
          title={t`This game is no longer in your library`}
          action={
            <Button asChild>
              <Link to={ROUTES.games}>{t`Back to My Games`}</Link>
            </Button>
          }
        />
      </div>
    );
  }

  const { game } = detail;
  const list = snapshots.data;
  const copies = summarizeCopies(list ?? []);

  return (
    <div>
      <GameHero
        game={game}
        running={running}
        protection={{ local: copies.onPc > 0, cloud: copies.inDrive > 0 }}
        canSnapshot={(status?.locationCount ?? 0) > 0}
        onCreateSnapshot={() => setCreating(true)}
        tabs={
          <Tabs<DetailsTab>
            tone="stage"
            value={tab}
            onChange={setTab}
            items={[
              { id: "overview", label: t`Overview` },
              { id: "saves", label: t`Saves`, ...(list ? { count: list.length } : {}) },
              { id: "artwork", label: t`Artwork` },
              { id: "properties", label: t`Properties` },
            ]}
          />
        }
      />

      {(tab === "overview" || tab === "artwork") && (
        <GameOverview
          detail={detail}
          snapshots={list ?? []}
          onOpenSaves={() => setTab("saves")}
          onChangeArtwork={() => setTab("artwork")}
          onRestore={setRestoring}
          onDelete={setDeleting}
        />
      )}
      {tab === "saves" && (
        <GameSavesPanel
          game={game}
          running={running}
          snapshots={list}
          onCreate={() => setCreating(true)}
          onRestore={setRestoring}
          onDelete={setDeleting}
        />
      )}
      {tab === "properties" && <PropertiesPanel key={game.title} detail={detail} />}

      <ArtworkDialog game={game} open={tab === "artwork"} onOpenChange={(open) => !open && setTab("overview")} />
      <CreateSnapshotDialog game={game} running={running} open={creating} onOpenChange={setCreating} />
      <RestoreSnapshotDialog snapshot={restoring} running={running} onClose={() => setRestoring(null)} />
      <DeleteSnapshotDialog snapshot={deleting} onClose={() => setDeleting(null)} />
    </div>
  );
}
