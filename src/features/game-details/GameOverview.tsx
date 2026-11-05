import { useLingui } from "@lingui/react/macro";

import { DsIcon } from "@/components/ds/ds-icon";
import { SectionHeader } from "@/components/ds/page-blocks";
import { StatusBadge } from "@/components/ds/status-badge";
import { StorageTags } from "@/components/ds/storage-tags";
import { GameArt } from "@/components/game/GameArt";
import { usePlatformNames } from "@/components/game/PlatformTag";
import { Button } from "@/components/ui/button";
import { SnapshotTimeline } from "@/features/saves/SnapshotTimeline";
import { useSnapshotLabels } from "@/features/saves/snapshot-labels";
import { summarizeCopies } from "@/features/saves/snapshot-summary";
import { useCurrentSave, useSaveLocations } from "@/features/saves/use-saves";
import type { GameDetail } from "@/lib/bindings/GameDetail";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";
import { formatBytes, formatDateTime, formatPlaytime, formatRelative } from "@/lib/format";

const RECENT_COUNT = 4;

interface GameOverviewProps {
  detail: GameDetail;
  snapshots: readonly SnapshotSummary[];
  onOpenSaves: () => void;
  onChangeArtwork: () => void;
  onRestore: (snapshot: SnapshotSummary) => void;
  onDelete: (snapshot: SnapshotSummary) => void;
}

/** Overview tab (design system "Game overview"): protection summary, recent snapshots, details. */
export function GameOverview({
  detail,
  snapshots,
  onOpenSaves,
  onChangeArtwork,
  onRestore,
  onDelete,
}: GameOverviewProps) {
  const { t, i18n } = useLingui();
  const labels = useSnapshotLabels();
  const platformNames = usePlatformNames();
  const { game } = detail;
  const current = useCurrentSave(game.id).data;
  const locations = useSaveLocations(game.id).data ?? [];
  const copies = summarizeCopies(snapshots);
  const latest = snapshots[0];
  const location = locations[0];

  const currentLine = (() => {
    if (!current || locations.length === 0) return null;
    if (current.latestSnapshotId === null) return { ok: false, text: t`No snapshot yet` };
    if (current.matchesLatest) return { ok: true, text: t`Matches the last snapshot` };
    return { ok: false, text: t`Changed since the last snapshot` };
  })();

  const details: [string, string][] = [
    [t`Platform`, platformNames[game.platform] ?? game.platform],
    [t`Playtime`, formatPlaytime(game.playtimeSeconds)],
    [t`Last played`, game.lastPlayedAt ? formatDateTime(game.lastPlayedAt, i18n.locale) : "—"],
    [t`Install size`, game.installed ? formatBytes(game.sizeBytes, i18n.locale) : "—"],
    [t`Developer`, detail.developer ?? "—"],
    [t`Released`, detail.releaseYear === null ? "—" : String(detail.releaseYear)],
  ];

  return (
    <div className="gd-ov">
      <div>
        <div className="gd-prot">
          <div>
            <div className="l8-eyebrow">{t`Last snapshot`}</div>
            <div className="v">{latest ? formatRelative(latest.createdAt, i18n.locale) : "—"}</div>
            <div className="s">
              {latest ? `${labels.kind(latest.kind)} · ${labels.name(latest)}` : t`No snapshot yet`}
            </div>
          </div>
          <div>
            <div className="l8-eyebrow">{t`Current save`}</div>
            <div className="v">
              {current && locations.length > 0
                ? labels.contents(current.totalFiles, formatBytes(current.totalBytes, i18n.locale))
                : "—"}
            </div>
            {currentLine ? (
              <div className={currentLine.ok ? "s ok" : "s warn"}>
                <DsIcon name={currentLine.ok ? "shieldCheck" : "alert"} size={16} />
                {currentLine.text}
              </div>
            ) : (
              <div className="s warn">
                <DsIcon name="alert" size={16} />
                {t`Not protected yet`}
              </div>
            )}
          </div>
          <div>
            <div className="l8-eyebrow">{t`Copies`}</div>
            <div className="v">{copies.total === 1 ? t`1 snapshot` : t`${copies.total} snapshots`}</div>
            <div className="s">{t`${copies.onPc} on PC · ${copies.inDrive} in Drive`}</div>
            <div className="s">
              <StorageTags local={copies.onPc > 0} cloud={copies.inDrive > 0} />
            </div>
          </div>
        </div>
        <SectionHeader
          title={t`Recent snapshots`}
          count={snapshots.length}
          action={
            <Button variant="ghost" size="sm" iconRight="arrowRight" onClick={onOpenSaves}>
              {t`Open Saves`}
            </Button>
          }
        />
        {snapshots.length > 0 ? (
          <SnapshotTimeline
            snapshots={snapshots.slice(0, RECENT_COUNT)}
            currentId={current?.matchesLatest ? current.latestSnapshotId : null}
            onRestore={onRestore}
            onDelete={onDelete}
          />
        ) : (
          <p className="m-0 rounded-md bg-surface-200 p-4 text-[13.5px] text-ink-muted">
            {t`No snapshots yet. A snapshot is taken automatically each time you stop playing.`}
          </p>
        )}
      </div>

      <aside className="gd-side">
        <h2>{t`Details`}</h2>
        <dl className="gd-dl">
          {details.map(([label, value]) => (
            <div key={label}>
              <dt>{label}</dt>
              <dd>{value}</dd>
            </div>
          ))}
        </dl>
        <h2>{t`Save location`}</h2>
        {location ? (
          <>
            <StatusBadge status={location.source === "user" ? "info" : "verified"}>
              {location.source === "user" ? t`Added by you` : t`Detected automatically`}
            </StatusBadge>
            <div className="gd-path">{location.path}</div>
          </>
        ) : (
          <StatusBadge status="unprotected" />
        )}
        <div className="gd-row">
          <Button size="sm" variant="ghost" icon="edit" onClick={onOpenSaves}>
            {location ? t`Change` : t`Add a save folder`}
          </Button>
        </div>
        <h2>{t`Artwork`}</h2>
        <div className="gd-artstrip">
          <div className="cv">
            <GameArt game={game} kind="cover" showTitle={false} />
          </div>
          <div>
            <GameArt game={game} kind="hero" showTitle={false} />
          </div>
          <div className="lg">
            <GameArt game={game} kind="logo" />
          </div>
        </div>
        <div className="gd-row">
          <Button size="sm" icon="image" onClick={onChangeArtwork}>
            {t`Change artwork`}
          </Button>
          <span className="l8-eyebrow" style={{ marginLeft: "auto" }}>
            SteamGridDB
          </span>
        </div>
      </aside>
    </div>
  );
}
