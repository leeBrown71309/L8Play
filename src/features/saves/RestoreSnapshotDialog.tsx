import { Trans, useLingui } from "@lingui/react/macro";

import { DsIcon } from "@/components/ds/ds-icon";
import { Banner } from "@/components/ds/page-blocks";
import { StorageTags } from "@/components/ds/storage-tags";
import { notify } from "@/components/feedback/toast-store";
import { Button } from "@/components/ui/button";
import { Dialog, Kbd } from "@/components/ui/dialog";
import { useGames } from "@/features/library/use-library";
import { useSnapshotLabels } from "@/features/saves/snapshot-labels";
import { useCurrentSave, useRestoreSnapshot, useSaveLocations } from "@/features/saves/use-saves";
import "@/features/saves/snapshot-dialogs.css";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";
import { formatBytes, formatDateTime, formatPlaytime } from "@/lib/format";

interface RestoreSnapshotDialogProps {
  snapshot: SnapshotSummary | null;
  running: boolean;
  onClose: () => void;
}

/** "0b77c9d1…" shortened for display. */
function shortHash(hash: string): string {
  return hash.length > 10 ? `${hash.slice(0, 4)}…${hash.slice(-4)}` : hash;
}

/** Shows what replaces what, and that the current save is protected first (design system "Restore"). */
export function RestoreSnapshotDialog({ snapshot, running, onClose }: RestoreSnapshotDialogProps) {
  const { t, i18n } = useLingui();
  const labels = useSnapshotLabels();
  const gameId = snapshot?.gameId ?? -1;
  const current = useCurrentSave(gameId);
  const locations = useSaveLocations(gameId);
  const game = useGames().data?.find((entry) => entry.id === gameId);
  const restore = useRestoreSnapshot();
  if (!snapshot) return null;
  const name = labels.name(snapshot);
  const cloudOnly = snapshot.status === "missing_local" && snapshot.cloud === "synced";

  // How much play time the restore rolls back; only when both totals are known.
  const rolledBack = game && snapshot.playtimeSeconds !== null ? game.playtimeSeconds - snapshot.playtimeSeconds : 0;

  const confirm = () =>
    restore.mutate(snapshot.id, {
      onSuccess: (report) => {
        notify({
          tone: "success",
          title: t`Snapshot restored`,
          body: report.safetySnapshotId
            ? t`“${name}” is back in place. Your previous save was kept as a safety copy.`
            : t`“${name}” is back in place. Your previous save was already in a snapshot.`,
        });
        onClose();
      },
      onError: (error) => notify({ tone: "danger", title: t`Nothing was restored`, body: labels.error(error) }),
    });

  return (
    <Dialog
      open
      onOpenChange={(open) => !open && onClose()}
      tone="warning"
      width={600}
      icon={<DsIcon name="restore" size={22} />}
      title={t`Restore “${name}”?`}
      description={snapshot.gameTitle}
      hint={
        <>
          <Kbd>Esc</Kbd> {t`cancel`}
        </>
      }
      actions={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t`Cancel`}
          </Button>
          <Button
            variant="primary"
            size="lg"
            icon="restore"
            onClick={confirm}
            loading={restore.isPending}
            disabled={running}
          >
            {restore.isPending ? t`Restoring…` : t`Back up current save & restore`}
          </Button>
        </>
      }
    >
      <div className="sd-flow">
        <div className="sd-box">
          <div className="l8-eyebrow">{t`Snapshot to restore`}</div>
          <b>{name}</b>
          <div className="m">
            <span>{formatDateTime(snapshot.createdAt, i18n.locale)}</span>
            <span>
              {snapshot.playtimeSeconds !== null ? t`${formatPlaytime(snapshot.playtimeSeconds)} played · ` : ""}
              {formatBytes(snapshot.totalBytes, i18n.locale)}
            </span>
          </div>
          <StorageTags
            local={snapshot.status !== "missing_local"}
            cloud={snapshot.cloud === "synced" ? true : snapshot.cloud === "queued" ? "pending" : false}
          />
        </div>
        <div className="sd-arrow">
          <DsIcon name="arrowRight" size={22} />
        </div>
        <div className="sd-box">
          <div className="l8-eyebrow">{t`Replaces your current save`}</div>
          <b>
            {current.data
              ? labels.contents(current.data.totalFiles, formatBytes(current.data.totalBytes, i18n.locale))
              : t`Reading the current save…`}
          </b>
          <div className="m">
            {current.data?.lastModifiedAt ? (
              <span>{formatDateTime(current.data.lastModifiedAt, i18n.locale)}</span>
            ) : null}
            {game ? <span className="w">{t`${formatPlaytime(game.playtimeSeconds)} played`}</span> : null}
          </div>
          {locations.data?.map((location) => (
            <div key={location.id} className="path">
              {location.path}
            </div>
          ))}
        </div>
      </div>

      {rolledBack >= 60 ? (
        <div className="sd-loss">
          <DsIcon name="alert" size={18} />
          <span>
            <Trans>
              The game will go back <b>{formatPlaytime(rolledBack)} of play</b>, to{" "}
              {formatDateTime(snapshot.createdAt, i18n.locale)}. Anything you did after that is only in your current
              save.
            </Trans>
          </span>
        </div>
      ) : null}

      <Banner tone="success" icon="shieldCheck" title={t`Your current save is backed up first.`}>
        {t`L8Play keeps “Safety copy before restore” on this PC, so you can undo this by restoring it from Saves.`}
      </Banner>

      <div className="sd-checks">
        <div>
          <DsIcon name={running ? "alert" : "checkCircle"} size={16} className={running ? "warn" : "ok"} />
          {running ? t`Close ${snapshot.gameTitle} before restoring` : t`${snapshot.gameTitle} is not running`}
        </div>
        <div>
          <DsIcon name="checkCircle" size={16} className="ok" />
          {cloudOnly
            ? t`The Drive copy is downloaded and checked against its checksum first`
            : t`Snapshot verified (SHA-256 ${shortHash(snapshot.archiveSha256)})`}
        </div>
        {game?.platform === "steam" ? (
          <div>
            <DsIcon name="info" size={16} className="inf" />
            {t`Steam Cloud may upload the restored save the next time the game starts`}
          </div>
        ) : null}
      </div>
    </Dialog>
  );
}
