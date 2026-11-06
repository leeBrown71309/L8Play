import { useLingui } from "@lingui/react/macro";
import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router";

import { ROUTES } from "@/app/routes";
import { DsIcon } from "@/components/ds/ds-icon";
import { SectionHeader } from "@/components/ds/page-blocks";
import { EmptyState } from "@/components/feedback/EmptyState";
import { GameTile } from "@/components/game/GameTile";
import { Button } from "@/components/ui/button";
import { IconButton } from "@/components/ui/icon-button";
import { buildAttention } from "@/features/dashboard/dashboard-model";
import {
  AttentionRows,
  BackingUpSection,
  RecentBackups,
  SaveProtectionSection,
} from "@/features/dashboard/dashboard-sections";
import { useAmbient } from "@/components/shell/ambient-store";
import { useCarouselAutoplay } from "@/features/dashboard/use-carousel-autoplay";
import { HeroCard } from "@/features/dashboard/HeroCard";
import { useWideScreen } from "@/features/dashboard/use-wide-screen";
import { useCloudStatus, useConflicts, useSyncOperations } from "@/features/cloud/use-cloud";
import { useArtworkLoading } from "@/features/library/artwork-loading-store";
import { recentlyPlayed } from "@/features/library/filter-games";
import { useGameActions } from "@/features/library/use-game-actions";
import { useGameProtection } from "@/features/library/use-game-protection";
import { useGames, useRunningGames } from "@/features/library/use-library";
import { useAddGameFlow, useScanFlow } from "@/features/library/use-library-flows";
import { useSaveStatuses } from "@/features/saves/use-saves";

/** Games the hero card can cycle through. */
const HERO_PAGES = 4;
/** Recently played tiles and attention rows: the 1366 layout shows 4 and 3, the wide one 6 and 4. */
const TILES = { narrow: 4, wide: 6 } as const;
const ATTENTION_ROWS = { narrow: 3, wide: 4 } as const;

function WelcomeCard() {
  const { t } = useLingui();
  const scan = useScanFlow();
  const addGame = useAddGameFlow();
  return (
    <section className="l8-herocard h-[262px] min-h-0">
      <div className="l8-herocard-body max-w-[480px]">
        <div className="l8-herocard-chips">
          <span className="l8-herocard-chip">
            <DsIcon name="shieldCheck" size={14} />
            {t`Welcome to L8Play`}
          </span>
        </div>
        <h2 className="l8-herocard-title">{t`Your entire gaming world. One place.`}</h2>
        <p className="l8-herocard-text">
          {t`No game was found automatically. Scan again after installing games, or add a folder or an .exe file.`}
        </p>
        <div className="l8-herocard-foot">
          <Button variant="light" size="lg" icon="sync" onClick={scan.start} disabled={scan.isScanning}>
            {t`Scan my libraries`}
          </Button>
          <Button variant="stage" size="lg" icon="plus" onClick={() => void addGame.start()}>
            {t`Add game`}
          </Button>
        </div>
      </div>
    </section>
  );
}

export function DashboardPage() {
  const { t } = useLingui();
  const { data: games, isPending } = useGames();
  const statuses = useSaveStatuses().data ?? [];
  const running = useRunningGames();
  const actions = useGameActions();
  const protection = useGameProtection();
  const loadingArt = useArtworkLoading((state) => state.pending);
  const hasAccount = useCloudStatus().data?.account != null;
  const operations = useSyncOperations(hasAccount).data ?? [];
  const conflicts = useConflicts(hasAccount).data ?? [];
  const navigate = useNavigate();
  const wide = useWideScreen();
  const tileCount = wide ? TILES.wide : TILES.narrow;
  const [page, setPage] = useState(0);
  const [paused, setPaused] = useState(false);
  const setAmbient = useAmbient((state) => state.setGame);

  const all = games ?? [];
  const recent = recentlyPlayed(
    all.filter((game) => game.installed),
    tileCount + 1,
  );
  const pages = Math.min(HERO_PAGES, recent.length);
  const featured = recent[Math.min(page, Math.max(pages - 1, 0))];
  const shownPage = Math.min(page, Math.max(pages - 1, 0));
  useCarouselAutoplay({ count: pages, page: shownPage, paused, onAdvance: setPage });

  // The page background follows the game on the hero card; it is released when leaving the dashboard.
  useEffect(() => {
    setAmbient(featured ?? null);
  }, [featured, setAmbient]);
  useEffect(() => () => setAmbient(null), [setAmbient]);

  const tiles = recent.filter((game) => game.id !== featured?.id).slice(0, tileCount);
  const attention = buildAttention({
    games: all,
    statuses,
    conflicts,
    operations,
    limit: wide ? ATTENTION_ROWS.wide : ATTENTION_ROWS.narrow,
  });
  const protectedGames = statuses.filter((status) => status.snapshotCount > 0).length;

  return (
    <div className="grid grid-cols-[minmax(0,1fr)_330px] gap-5 min-[1700px]:grid-cols-[minmax(0,1fr)_400px] @max-[880px]/page:grid-cols-1">
      <div className="flex min-w-0 flex-col gap-[22px]">
        {isPending ? (
          <div className="l8-skel h-[262px] rounded-lg" />
        ) : featured ? (
          <HeroCard
            game={featured}
            protection={protection.of(featured.id)}
            running={running.has(featured.id)}
            loading={loadingArt.has(featured.id)}
            pages={pages}
            page={shownPage}
            onPage={setPage}
            onPauseChange={setPaused}
            onPlay={() => actions.play(featured)}
          />
        ) : (
          <WelcomeCard />
        )}

        <section>
          <SectionHeader
            title={t`Recently played`}
            action={
              <Button asChild variant="ghost" size="sm">
                <Link to={ROUTES.games}>{t`See all`}</Link>
              </Button>
            }
          />
          {tiles.length > 0 ? (
            // The tiles follow the width of their column, not the window: 4 across, 2 when narrow, 1 when very narrow.
            <div className="@container">
              <div className="relative grid grid-cols-4 gap-4 @max-[640px]:grid-cols-2 @max-[300px]:grid-cols-1 min-[1700px]:grid-cols-6 min-[1700px]:gap-[18px] [&_.l8-tile-art]:aspect-[1/0.92]">
                {tiles.map((game, index) => (
                  <GameTile
                    key={game.id}
                    game={game}
                    running={running.has(game.id)}
                    loading={loadingArt.has(game.id)}
                    protection={protection.of(game.id)}
                    selected={index === 0}
                    onPlay={actions.play}
                  />
                ))}
                <IconButton
                  icon="chevronRight"
                  label={t`More games`}
                  className="absolute top-1/2 -right-3.5 z-[3] -mt-5 bg-on-stage text-[#2a1215] shadow-2 hover:bg-white hover:text-[#2a1215]"
                  onClick={() => void navigate(ROUTES.games)}
                />
              </div>
            </div>
          ) : (
            <div className="rounded-lg bg-surface-200">
              <EmptyState
                icon={<DsIcon name="gamepad" size={32} />}
                title={t`Nothing played recently`}
                body={t`Games you play will show up here, most recent first.`}
              />
            </div>
          )}
        </section>

        <BackingUpSection operations={operations} games={all} />
        {wide ? <RecentBackups snapshots={protection.snapshots} games={all} /> : null}
      </div>

      <div className="flex min-w-0 flex-col">
        <AttentionRows items={attention} />
        <div className="mt-[22px] flex flex-1 flex-col">
          <SaveProtectionSection
            snapshots={protection.snapshots}
            protectedGames={protectedGames}
            totalGames={all.length}
          />
        </div>
      </div>
    </div>
  );
}
