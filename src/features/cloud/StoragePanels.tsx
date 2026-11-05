import { useLingui } from "@lingui/react/macro";
import { useMemo } from "react";

import { StorageMeter } from "@/components/ds/progress-bar";
import { SectionHeader } from "@/components/ds/page-blocks";
import { GameArt } from "@/components/game/GameArt";
import type { CloudQuota } from "@/lib/bindings/CloudQuota";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";
import { formatBytes } from "@/lib/format";

interface DriveFolder {
  game: GameSummary | undefined;
  gameId: number;
  title: string;
  snapshots: number;
  bytes: number;
}

/** One folder per game, built from the snapshots that have a copy in Google Drive. */
function useDriveFolders(snapshots: readonly SnapshotSummary[], games: readonly GameSummary[]): DriveFolder[] {
  return useMemo(() => {
    const byGame = new Map<number, DriveFolder>();
    for (const snapshot of snapshots) {
      if (snapshot.cloud !== "synced") continue;
      const folder = byGame.get(snapshot.gameId) ?? {
        game: games.find((game) => game.id === snapshot.gameId),
        gameId: snapshot.gameId,
        title: snapshot.gameTitle,
        snapshots: 0,
        bytes: 0,
      };
      folder.snapshots += 1;
      folder.bytes += snapshot.archiveSizeBytes;
      byGame.set(snapshot.gameId, folder);
    }
    return [...byGame.values()].sort((a, b) => b.bytes - a.bytes);
  }, [snapshots, games]);
}

interface StorageCardProps {
  quota: CloudQuota | undefined;
  snapshots: readonly SnapshotSummary[];
}

/** Drive storage: the used figure and a meter split into L8Play and everything else. */
export function StorageCard({ quota, snapshots }: StorageCardProps) {
  const { t, i18n } = useLingui();
  const locale = i18n.locale;
  const l8Bytes = snapshots.reduce(
    (sum, snapshot) => (snapshot.cloud === "synced" ? sum + snapshot.archiveSizeBytes : sum),
    0,
  );
  const used = quota?.usedBytes ?? 0;
  const limit = quota?.limitBytes ?? null;
  const ours = Math.min(l8Bytes, used);
  const total = limit ?? Math.max(used, 1);

  return (
    <section className="cloud-store">
      <div className="l8-eyebrow">{t`Drive storage`}</div>
      <div className="big">
        <b>{quota ? formatBytes(used, locale) : "—"}</b>
        {limit ? <span>{t`of ${formatBytes(limit, locale)}`}</span> : quota ? <span>{t`used`}</span> : null}
      </div>
      {quota ? (
        <StorageMeter
          total={total}
          summary={
            limit
              ? t`${formatBytes(used, locale)} of ${formatBytes(limit, locale)} used`
              : t`${formatBytes(used, locale)} used`
          }
          freeText={t`Free`}
          {...(limit ? { freeLabel: formatBytes(Math.max(limit - used, 0), locale) } : {})}
          segments={[
            { label: "L8Play", value: ours, display: formatBytes(ours, locale), tone: "flare" },
            { label: t`Other files`, value: used - ours, display: formatBytes(used - ours, locale), tone: "other" },
          ]}
        />
      ) : (
        <p className="cloud-empty" style={{ padding: 0 }}>
          {t`Storage figures appear once Google answers.`}
        </p>
      )}
    </section>
  );
}

interface DriveFoldersProps {
  snapshots: readonly SnapshotSummary[];
  games: readonly GameSummary[];
}

/** "In your Drive": the per-game folders L8Play keeps there. */
export function DriveFolders({ snapshots, games }: DriveFoldersProps) {
  const { t, i18n } = useLingui();
  const folders = useDriveFolders(snapshots, games);
  return (
    <section className="cloud-browser">
      <SectionHeader title={t`In your Drive`} />
      {folders.length === 0 ? (
        <p className="cloud-empty">{t`No snapshot is in Drive yet.`}</p>
      ) : (
        folders.map((folder) => (
          <div key={folder.gameId} className="cloud-folder">
            <span className="ic">
              {folder.game ? <GameArt game={folder.game} kind="icon" showTitle={false} /> : null}
            </span>
            <span className="t">
              {folder.title}
              <span>{folder.snapshots === 1 ? t`1 snapshot` : t`${folder.snapshots} snapshots`}</span>
            </span>
            <span className="s">{formatBytes(folder.bytes, i18n.locale)}</span>
          </div>
        ))
      )}
    </section>
  );
}
