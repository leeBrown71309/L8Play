import { useLingui } from "@lingui/react/macro";
import { useEffect } from "react";

import { DsIcon } from "@/components/ds/ds-icon";
import { PageHeader } from "@/components/ds/page-blocks";
import { EmptyState } from "@/components/feedback/EmptyState";
import { GameCard } from "@/components/game/GameCard";
import { GameRow, GameRowHeader } from "@/components/game/GameRow";
import { Button } from "@/components/ui/button";
import { useArtworkLoading } from "@/features/library/artwork-loading-store";
import { filterGames } from "@/features/library/filter-games";
import { LibraryPager } from "@/features/library/LibraryPager";
import { clampPage, PAGE_SIZES, pageSlice } from "@/features/library/pagination";
import { LibraryToolbar } from "@/features/library/LibraryToolbar";
import { useLibraryView } from "@/features/library/library-view-store";
import { useGameActions } from "@/features/library/use-game-actions";
import { useGameProtection } from "@/features/library/use-game-protection";
import { useGames, useRunningGames } from "@/features/library/use-library";
import { useAddGameFlow, useScanFlow } from "@/features/library/use-library-flows";
import { cn } from "@/lib/cn";

const GRID = "grid grid-cols-[repeat(auto-fill,minmax(var(--l8-cover-w),1fr))] gap-x-4 gap-y-6";

export function LibraryPage() {
  const { t } = useLingui();
  const { data: games, isPending, isError, refetch } = useGames();
  const running = useRunningGames();
  const view = useLibraryView();
  const actions = useGameActions();
  const scan = useScanFlow();
  const addGame = useAddGameFlow();
  const protection = useGameProtection();
  const loadingArt = useArtworkLoading((state) => state.pending);
  const visible = games ? filterGames(games, view) : [];
  // The stored page can outlive a smaller result (filters, removed games): show the nearest valid one.
  const page = clampPage(view.page, visible.length, view.pageSize);
  const shown = pageSlice(visible, page, view.pageSize);
  // Small libraries fit the smallest page size: no pager to show.
  const needsPager = visible.length > PAGE_SIZES[0];

  // A new page starts at the top of the scrolling area.
  useEffect(() => {
    document.querySelector(".l8-main-body")?.scrollTo({ top: 0 });
  }, [page, view.pageSize]);

  const header = (
    <PageHeader
      title={t`My Games`}
      {...(games ? { count: visible.length } : {})}
      actions={
        <>
          <Button
            variant="ghost"
            icon={<DsIcon name="sync" size={18} className={cn(scan.isScanning && "animate-spin")} />}
            onClick={scan.start}
            disabled={scan.isScanning}
          >
            {scan.isScanning ? t`Scanning…` : t`Scan libraries`}
          </Button>
          <Button icon="plus" onClick={() => void addGame.start()} disabled={addGame.isAdding}>
            {t`Add game`}
          </Button>
        </>
      }
    />
  );

  if (isPending) {
    return (
      <>
        {header}
        <div className={GRID}>
          {Array.from({ length: 12 }, (_, index) => (
            <div key={index} className="l8-card">
              <div className="l8-card-frame">
                <div className="l8-card-art">
                  <div className="l8-skel l8-card-skel" />
                </div>
              </div>
              <div className="l8-card-meta">
                <div className="l8-skel l8-skel-line" />
                <div className="l8-skel l8-skel-line is-short" />
              </div>
            </div>
          ))}
        </div>
      </>
    );
  }

  if (isError) {
    return (
      <>
        {header}
        <div className="rounded-lg bg-surface-200">
          <EmptyState
            icon={<DsIcon name="alert" size={32} />}
            title={t`Your library could not be loaded`}
            action={<Button onClick={() => void refetch()}>{t`Try again`}</Button>}
          />
        </div>
      </>
    );
  }

  if (games.length === 0) {
    return (
      <>
        {header}
        <div className="rounded-lg bg-surface-200">
          <EmptyState
            icon={<DsIcon name="gamepad" size={32} />}
            title={t`No games found yet`}
            body={t`Steam, Epic Games, Xbox and GOG installations are detected automatically. Add other games with a folder or an .exe file.`}
            action={
              <>
                <Button variant="primary" icon="sync" onClick={scan.start} disabled={scan.isScanning}>
                  {t`Scan libraries`}
                </Button>
                <Button icon="plus" onClick={() => void addGame.start()}>
                  {t`Add game`}
                </Button>
              </>
            }
          />
        </div>
      </>
    );
  }

  return (
    <>
      {header}
      <LibraryToolbar games={games} />
      {visible.length === 0 ? (
        <div className="rounded-lg bg-surface-200">
          <EmptyState
            icon={<DsIcon name="search" size={32} />}
            title={t`No game matches these filters`}
            action={
              <Button
                onClick={() => view.setFilters({ search: "", platform: "all", install: "all", favoritesOnly: false })}
              >
                {t`Clear filters`}
              </Button>
            }
          />
        </div>
      ) : view.view === "grid" ? (
        <div className={cn(GRID, "pt-2")}>
          {shown.map((game) => (
            <GameCard
              key={game.id}
              game={game}
              running={running.has(game.id)}
              loading={loadingArt.has(game.id)}
              protection={protection.of(game.id)}
              onPlay={actions.play}
              onToggleFavorite={actions.toggleFavorite}
            />
          ))}
        </div>
      ) : (
        <div role="table" aria-label={t`My Games`} className="-mx-3">
          <GameRowHeader sort={view.sort} />
          {shown.map((game) => (
            <GameRow
              key={game.id}
              game={game}
              running={running.has(game.id)}
              loading={loadingArt.has(game.id)}
              protection={protection.of(game.id)}
              onPlay={actions.play}
            />
          ))}
        </div>
      )}
      {needsPager ? (
        <LibraryPager
          total={visible.length}
          page={page}
          pageSize={view.pageSize}
          onPage={view.setPage}
          onPageSize={view.setPageSize}
        />
      ) : null}
    </>
  );
}
