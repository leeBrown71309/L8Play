import { useLingui } from "@lingui/react/macro";
import { useNavigate } from "react-router";

import { ROUTES } from "@/app/routes";
import { ActionRow, ProtectionOrb, SectionHeader, TransferRow } from "@/components/ds/page-blocks";
import { StorageTags } from "@/components/ds/storage-tags";
import { GameArt } from "@/components/game/GameArt";
import { Button } from "@/components/ui/button";
import { IconButton } from "@/components/ui/icon-button";
import { activeUploads, transferPercent, type AttentionItem } from "@/features/dashboard/dashboard-model";
import { protectionTotals } from "@/features/library/game-protection";
import { useSnapshotLabels } from "@/features/saves/snapshot-labels";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";
import type { SyncOperationInfo } from "@/lib/bindings/SyncOperationInfo";
import { formatBytes, formatDateTime, formatPlaytime } from "@/lib/format";

/** Games that need a decision or an eye: conflicts, uploads, unprotected saves. */
export function AttentionRows({ items }: { items: readonly AttentionItem[] }) {
  const { t, i18n } = useLingui();
  const navigate = useNavigate();
  return (
    <section>
      {items.map((item) => {
        const aside = {
          conflict: t`Sync conflict`,
          uploading: t`Uploading`,
          queued: t`Queued`,
          unprotected: t`Not protected`,
          "no-snapshot": t`No snapshot yet`,
        }[item.kind];
        const played = formatPlaytime(item.game.playtimeSeconds);
        const description = {
          conflict: t`Drive has a different save from ${item.remoteDevice ?? ""}`,
          uploading: t`${item.snapshotName ?? ""} · ${formatBytes(item.bytesDone, i18n.locale)} of ${formatBytes(item.bytesTotal, i18n.locale)}`,
          queued: t`Snapshots waiting to upload: ${item.operations}`,
          unprotected: t`Save folder not found · ${played} played`,
          "no-snapshot": t`Save folder found, no snapshot yet · ${played} played`,
        }[item.kind];
        const target = item.kind === "conflict" ? ROUTES.cloud : `/games/${item.game.id}`;
        return (
          <ActionRow
            key={`${item.kind}-${item.game.id}`}
            game={item.game}
            title={item.game.title}
            aside={aside}
            description={description}
            onClick={() => void navigate(target)}
          />
        );
      })}
    </section>
  );
}

interface BackingUpProps {
  operations: readonly SyncOperationInfo[];
  games: readonly GameSummary[];
}

/** "Backing up now": the uploads running or waiting, with their progress. */
export function BackingUpSection({ operations, games }: BackingUpProps) {
  const { t, i18n } = useLingui();
  const navigate = useNavigate();
  const active = activeUploads(operations, 2);
  if (active.length === 0) return null;
  return (
    <section>
      <SectionHeader
        title={t`Backing up now`}
        action={
          <Button variant="ghost" size="sm" onClick={() => void navigate(ROUTES.saves)}>
            {t`Save Manager`}
          </Button>
        }
      />
      <div className="flex flex-col gap-2.5">
        {active.map((operation) => {
          const game = games.find((candidate) => candidate.title === operation.gameTitle);
          if (!game) return null;
          const running = operation.status === "running";
          const percent = transferPercent(operation.bytesDone, operation.bytesTotal);
          return (
            <TransferRow
              key={operation.id}
              game={game}
              chip={operation.snapshotName}
              status={running ? t`Uploading to Drive · ${percent}%` : t`Waiting to upload`}
              meta={`${formatBytes(operation.bytesDone, i18n.locale)} / ${formatBytes(operation.bytesTotal, i18n.locale)}`}
              value={running ? percent : 0}
              tone={running ? "cloud" : "warning"}
              pauseLabel={t`Pause`}
              cancelLabel={t`Cancel`}
            />
          );
        })}
      </div>
    </section>
  );
}

interface ProtectionSectionProps {
  snapshots: readonly SnapshotSummary[];
  protectedGames: number;
  totalGames: number;
}

/** "Save protection": the orb with the totals, and the PC / Drive / Queued bubbles. */
export function SaveProtectionSection({ snapshots, protectedGames, totalGames }: ProtectionSectionProps) {
  const { t } = useLingui();
  const navigate = useNavigate();
  const totals = protectionTotals(snapshots);
  return (
    <section className="flex flex-1 flex-col">
      <SectionHeader
        title={t`Save protection`}
        action={
          <IconButton
            icon="arrowRight"
            label={t`Open Save Manager`}
            size="sm"
            onClick={() => void navigate(ROUTES.saves)}
          />
        }
      />
      <div className="flex flex-1 flex-col items-center justify-center rounded-lg bg-[linear-gradient(180deg,var(--l8-bg-200)_0%,var(--l8-bg-000)_100%)] px-4 pt-5 pb-[22px] [&_.l8-orb-core]:size-[136px] [&_.l8-orb-stage]:size-[210px] min-[1700px]:[&_.l8-orb-core]:size-[168px] min-[1700px]:[&_.l8-orb-stage]:size-[260px]">
        <ProtectionOrb
          value={totals.total}
          label={t`Snapshots`}
          sub={t`${protectedGames} of ${totalGames} games`}
          bubbles={[
            { icon: "drive", value: totals.onPc, label: t`This PC`, tone: "cream" },
            { icon: "cloud", value: totals.inDrive, label: t`Drive`, tone: "cloud" },
            { icon: "clock", value: totals.queued, label: t`Queued`, tone: "flare" },
          ]}
        />
      </div>
    </section>
  );
}

/** The newest snapshots as small cards; the grid wraps as the window narrows and the caller picks how many. */
export function RecentBackups({
  snapshots,
  games,
  limit,
}: {
  snapshots: readonly SnapshotSummary[];
  games: readonly GameSummary[];
  limit: number;
}) {
  const { t, i18n } = useLingui();
  const navigate = useNavigate();
  const labels = useSnapshotLabels();
  const recent = [...snapshots].sort((a, b) => b.createdAt.localeCompare(a.createdAt)).slice(0, limit);
  if (recent.length === 0) return null;
  return (
    <section>
      <SectionHeader
        title={t`Recent backups`}
        action={
          <Button variant="ghost" size="sm" onClick={() => void navigate(ROUTES.saves)}>
            {t`Save Manager`}
          </Button>
        }
      />
      <div className="mb-3.5 grid grid-cols-[repeat(auto-fit,minmax(200px,1fr))] gap-3.5">
        {recent.map((snapshot) => {
          const game = games.find((candidate) => candidate.id === snapshot.gameId);
          return (
            <article key={snapshot.id} className="l8-snapcard">
              <div className="l8-snapcard-shot">
                {game ? <GameArt game={game} kind="hero" showTitle={false} /> : null}
                <span className="l8-snapcard-time">{formatDateTime(snapshot.createdAt, i18n.locale)}</span>
              </div>
              <div className="l8-snapcard-body">
                <div className="l8-snapcard-game">{snapshot.gameTitle}</div>
                <div className="l8-snapcard-name" title={labels.name(snapshot)}>
                  {labels.name(snapshot)}
                </div>
                <div className="l8-snapcard-meta">
                  <StorageTags
                    local={snapshot.archivePath !== null}
                    cloud={
                      snapshot.cloud === "synced"
                        ? true
                        : snapshot.cloud === "none" || snapshot.cloud === "failed"
                          ? false
                          : "pending"
                    }
                    compact
                  />
                  <span>{formatBytes(snapshot.archiveSizeBytes, i18n.locale)}</span>
                </div>
              </div>
            </article>
          );
        })}
      </div>
    </section>
  );
}
