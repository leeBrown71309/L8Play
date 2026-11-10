import { useLingui } from "@lingui/react/macro";
import { useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Link } from "react-router";

import { ROUTES } from "@/app/routes";
import { DsIcon } from "@/components/ds/ds-icon";
import { Dropdown } from "@/components/ds/form-fields";
import { PageHeader } from "@/components/ds/page-blocks";
import { EmptyState } from "@/components/feedback/EmptyState";
import { Skeleton } from "@/components/feedback/Skeleton";
import { GameArt } from "@/components/game/GameArt";
import { SnapshotIcon } from "@/components/icons";
import { ActionMenu } from "@/components/ui/action-menu";
import { Button } from "@/components/ui/button";
import {
  filterSnapshots,
  type DateFilter,
  type KindFilter,
  type StoredFilter,
} from "@/features/save-manager/snapshot-filters";
import { SnapshotDetailPanel } from "@/features/save-manager/SnapshotDetailPanel";
import { useExportAllSaves } from "@/features/save-manager/use-export-saves";
import { useGames, useRunningGames } from "@/features/library/use-library";
import { useImportStore } from "@/features/saves-import/import-store";
import { CreateSnapshotDialog } from "@/features/saves/CreateSnapshotDialog";
import { DeleteSnapshotDialog } from "@/features/saves/DeleteSnapshotDialog";
import { RestoreSnapshotDialog } from "@/features/saves/RestoreSnapshotDialog";
import { SnapshotTimeline } from "@/features/saves/SnapshotTimeline";
import { useRevealSnapshot, useSaveStatuses, useSnapshots } from "@/features/saves/use-saves";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";
import "@/features/save-manager/save-manager-page.css";

/** Every snapshot, filterable by game, kind, storage place and date, with the selected one in detail. */
export function SaveManagerPage() {
  const { t } = useLingui();
  const games = useGames();
  const snapshots = useSnapshots(null);
  const statuses = useSaveStatuses();
  const reveal = useRevealSnapshot();
  const exporter = useExportAllSaves();
  const running = useRunningGames();
  const [gameFilter, setGameFilter] = useState<number | null>(null);
  const [kindFilter, setKindFilter] = useState<KindFilter>("all");
  const [storedFilter, setStoredFilter] = useState<StoredFilter>("any");
  const [dateFilter, setDateFilter] = useState<DateFilter>("all");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [restoring, setRestoring] = useState<SnapshotSummary | null>(null);
  const [deleting, setDeleting] = useState<SnapshotSummary | null>(null);
  const [creating, setCreating] = useState(false);
  // On narrow windows the details slide over the page; this says whether that panel is out.
  const [detailOpen, setDetailOpen] = useState(false);
  const grid = useRef<HTMLDivElement>(null);
  /** The detail column is hidden when the page is narrow (see the CSS): only then does the slide-over open. */
  const detailColumnHidden = () => {
    const column = grid.current?.querySelector(":scope > .sm-detail");
    return column ? getComputedStyle(column).display === "none" : false;
  };

  useEffect(() => {
    if (!detailOpen) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setDetailOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [detailOpen]);

  const all = useMemo(() => snapshots.data ?? [], [snapshots.data]);
  const visible = useMemo(
    () => filterSnapshots(all, { gameId: gameFilter, kind: kindFilter, stored: storedFilter, date: dateFilter }),
    [all, gameFilter, kindFilter, storedFilter, dateFilter],
  );

  const selected = visible.find((snapshot) => snapshot.id === selectedId) ?? visible[0];
  const gameList = useMemo(() => games.data ?? [], [games.data]);
  const selectedGame = selected ? gameList.find((game) => game.id === selected.gameId) : undefined;
  const filterGame = gameFilter === null ? undefined : gameList.find((game) => game.id === gameFilter);

  /** Games with at least one snapshot or save location, most recently protected first. */
  const protectedGames = useMemo(() => {
    const byId = new Map(gameList.map((game) => [game.id, game]));
    return (statuses.data ?? [])
      .filter((status) => status.snapshotCount > 0 || status.locationCount > 0)
      .flatMap((status) => {
        const game = byId.get(status.gameId);
        return game ? [{ game, status }] : [];
      })
      .sort((a, b) => (b.status.lastSnapshotAt ?? "").localeCompare(a.status.lastSnapshotAt ?? ""));
  }, [gameList, statuses.data]);

  const detailPanel = (
    <SnapshotDetailPanel
      snapshot={selected}
      game={selectedGame}
      onRestore={setRestoring}
      onDelete={setDeleting}
      onReveal={(snapshot) => reveal.mutate(snapshot.id)}
      onClose={() => setDetailOpen(false)}
    />
  );

  return (
    <>
      <div ref={grid} className="sm-grid">
        <nav className="sm-games" aria-label={t`Games`}>
          <div className="l8-eyebrow">{t`Games`}</div>
          <button
            type="button"
            className={`sm-gi${gameFilter === null ? " on" : ""}`}
            onClick={() => setGameFilter(null)}
          >
            <span className="ic all">
              <DsIcon name="layers" size={14} />
            </span>
            <span className="t">{t`All games`}</span>
            <span className="n">{all.length}</span>
          </button>
          <div className="sm-games-list">
            {protectedGames.map(({ game, status }) => (
              <button
                key={game.id}
                type="button"
                className={`sm-gi${gameFilter === game.id ? " on" : ""}`}
                onClick={() => setGameFilter(game.id)}
              >
                <span className="ic">
                  <GameArt game={game} kind="icon" showTitle={false} />
                </span>
                <span className="t">{game.title}</span>
                <span className="n">{status.snapshotCount}</span>
              </button>
            ))}
          </div>
        </nav>

        <div className="sm-list">
          <PageHeader
            title={t`Save Manager`}
            count={visible.length}
            actions={
              <>
                {filterGame && (
                  <Button asChild variant="ghost">
                    <Link to={`/games/${filterGame.id}`}>{t`Open game page`}</Link>
                  </Button>
                )}
                <ActionMenu
                  items={[
                    {
                      id: "create",
                      label: t`Create snapshot`,
                      icon: "plus",
                      disabled: !filterGame,
                      hint: t`Pick a game on the left first`,
                      onSelect: () => setCreating(true),
                    },
                    {
                      id: "import",
                      label: t`Import saves`,
                      icon: "upload",
                      onSelect: () => useImportStore.getState().open(),
                    },
                    {
                      id: "export",
                      label: exporter.exporting ? t`Exporting…` : t`Export all saves`,
                      icon: "download",
                      disabled: exporter.exporting,
                      onSelect: () => void exporter.run(),
                    },
                  ]}
                  trigger={({ open, toggle }) => (
                    <Button
                      variant="primary"
                      iconRight="chevronDown"
                      aria-haspopup="menu"
                      aria-expanded={open}
                      onClick={toggle}
                    >
                      {t`Actions`}
                    </Button>
                  )}
                />
              </>
            }
          />
          <div className="sm-bar">
            <Dropdown<KindFilter>
              prefix={t`Kind`}
              value={kindFilter}
              onChange={setKindFilter}
              options={[
                { id: "all", label: t`All` },
                { id: "manual", label: t`Manual` },
                { id: "automatic", label: t`Automatic` },
                { id: "safety", label: t`Safety copies` },
              ]}
            />
            <Dropdown<StoredFilter>
              prefix={t`Stored`}
              value={storedFilter}
              onChange={setStoredFilter}
              options={[
                { id: "any", label: t`Anywhere` },
                { id: "pc", label: t`On this PC` },
                { id: "drive", label: t`In Google Drive` },
                { id: "pc_only", label: t`Only on this PC` },
                { id: "drive_only", label: t`Only in Drive` },
              ]}
            />
            <Dropdown<DateFilter>
              prefix={t`Date`}
              value={dateFilter}
              onChange={setDateFilter}
              options={[
                { id: "all", label: t`All time` },
                { id: "7", label: t`Last 7 days` },
                { id: "30", label: t`Last 30 days` },
              ]}
            />
          </div>
          {snapshots.isPending ? (
            <Skeleton className="h-60 rounded-lg" />
          ) : visible.length === 0 ? (
            <div className="rounded-lg bg-surface-200">
              <EmptyState
                icon={<SnapshotIcon size={30} />}
                title={all.length === 0 ? t`No snapshots yet` : t`No snapshot matches these filters`}
                body={
                  all.length === 0
                    ? t`Play a game and quit it: L8Play copies its saves automatically. You can also create a snapshot from a game’s page.`
                    : undefined
                }
                action={
                  all.length === 0 ? (
                    <Button asChild variant="primary">
                      <Link to={ROUTES.games}>{t`Go to My Games`}</Link>
                    </Button>
                  ) : undefined
                }
              />
            </div>
          ) : (
            <SnapshotTimeline
              snapshots={visible}
              showGame={gameFilter === null}
              compact
              showNote={false}
              selectedId={selected?.id ?? null}
              onSelect={(snapshot) => {
                setSelectedId(snapshot.id);
                setDetailOpen(detailColumnHidden());
              }}
              onRestore={setRestoring}
              onDelete={setDeleting}
            />
          )}
        </div>

        {detailPanel}
      </div>
      {/*
        Narrow windows: the same panel slides over the page. It lives at the root of the document so the page's
        own animation (a transform) cannot trap it under the side rails.
      */}
      {createPortal(
        <div className={`sm-drawer${detailOpen ? " is-open" : ""}`}>
          <div className="sm-scrim" aria-hidden="true" onClick={() => setDetailOpen(false)} />
          {detailPanel}
        </div>,
        document.body,
      )}

      {filterGame && (
        <CreateSnapshotDialog
          game={filterGame}
          running={running.has(filterGame.id)}
          open={creating}
          onOpenChange={setCreating}
        />
      )}
      <RestoreSnapshotDialog
        snapshot={restoring}
        running={restoring ? running.has(restoring.gameId) : false}
        onClose={() => setRestoring(null)}
      />
      <DeleteSnapshotDialog snapshot={deleting} onClose={() => setDeleting(null)} />
    </>
  );
}
