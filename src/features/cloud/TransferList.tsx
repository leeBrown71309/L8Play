import { useLingui } from "@lingui/react/macro";

import { DsIcon } from "@/components/ds/ds-icon";
import { ProgressBar } from "@/components/ds/progress-bar";
import { SectionHeader } from "@/components/ds/page-blocks";
import { GameArt } from "@/components/game/GameArt";
import { Button } from "@/components/ui/button";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import type { SyncOperationInfo } from "@/lib/bindings/SyncOperationInfo";
import { cn } from "@/lib/cn";
import { formatBytes } from "@/lib/format";

interface TransferListProps {
  operations: readonly SyncOperationInfo[];
  games: readonly GameSummary[];
  canRetry: boolean;
  retrying: boolean;
  onRetry: () => void;
}

/** Live uploads and downloads as the design system's transfer strips, one per sync operation. */
export function TransferList({ operations, games, canRetry, retrying, onRetry }: TransferListProps) {
  const { t, i18n } = useLingui();
  const iconFor = (title: string) => games.find((game) => game.title === title);

  const describe = (operation: SyncOperationInfo) => {
    const percent = operation.bytesTotal > 0 ? Math.round((operation.bytesDone / operation.bytesTotal) * 100) : 0;
    const upload = operation.direction === "upload";
    switch (operation.status) {
      case "running":
        return {
          tone: "cloud" as const,
          value: percent,
          status: upload ? t`Uploading · ${percent}%` : t`Downloading · ${percent}%`,
          meta: t`${formatBytes(operation.bytesDone, i18n.locale)} of ${formatBytes(operation.bytesTotal, i18n.locale)}`,
        };
      case "queued":
        return {
          tone: "cloud" as const,
          status: upload ? t`Waiting to upload` : t`Waiting to download`,
          meta: formatBytes(operation.bytesTotal, i18n.locale),
        };
      case "paused":
        return { tone: "warning" as const, status: t`Paused`, meta: t`Sign-in needed` };
      case "failed":
        return {
          tone: "warning" as const,
          status: t`Failed`,
          meta: operation.lastError ?? t`It will be tried again`,
        };
    }
  };

  return (
    <section>
      <SectionHeader
        title={t`Transfers`}
        count={operations.length}
        action={
          canRetry ? (
            <Button variant="ghost" size="sm" disabled={retrying} onClick={onRetry}>
              {t`Retry failed`}
            </Button>
          ) : undefined
        }
      />
      {operations.length === 0 ? (
        <p className="cloud-empty">{t`Nothing is waiting to upload. New snapshots appear here while they are copied to Drive.`}</p>
      ) : (
        <div className="cloud-xfers">
          {operations.map((operation) => {
            const info = describe(operation);
            const game = iconFor(operation.gameTitle);
            return (
              <div key={operation.id} className={cn("l8-xfer", `tone-${info.tone}`)}>
                <span className="l8-xfer-tile">
                  {game ? <GameArt game={game} kind="icon" showTitle={false} /> : <DsIcon name="cloud" size={20} />}
                </span>
                <div className="l8-xfer-name">
                  <b>{operation.gameTitle}</b>
                  <span className="l8-xfer-chip">{operation.snapshotName || t`Automatic snapshot`}</span>
                </div>
                <div className="l8-xfer-prog">
                  <div className="l8-xfer-meta">
                    <b>{info.status}</b>
                    <span>{info.meta}</span>
                  </div>
                  {info.value !== undefined ? <ProgressBar value={info.value} tone={info.tone} size="sm" /> : null}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}
