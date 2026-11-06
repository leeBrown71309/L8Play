import { useLingui } from "@lingui/react/macro";

import { DsIcon } from "@/components/ds/ds-icon";
import { StatusBadge, type StatusKind } from "@/components/ds/status-badge";
import { GameArt } from "@/components/game/GameArt";
import { Button } from "@/components/ui/button";
import { IconButton } from "@/components/ui/icon-button";
import { useSnapshotLabels } from "@/features/saves/snapshot-labels";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import type { SnapshotKind } from "@/lib/bindings/SnapshotKind";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";
import { formatBytes, formatDateTime, formatPlaytime } from "@/lib/format";

const KIND_BADGE: Record<SnapshotKind, StatusKind> = {
  manual: "manual",
  auto_exit: "auto",
  auto_launch: "auto",
  scheduled: "auto",
  safety: "safety",
  conflict_copy: "safety",
};

/** `0b77c9d1…` → `0b77…c9d1`. */
function shortHash(hash: string): string {
  return hash.length > 12 ? `${hash.slice(0, 4)}…${hash.slice(-4)}` : hash;
}

interface SnapshotDetailPanelProps {
  snapshot: SnapshotSummary | undefined;
  game: GameSummary | undefined;
  onRestore: (snapshot: SnapshotSummary) => void;
  onDelete: (snapshot: SnapshotSummary) => void;
  onReveal: (snapshot: SnapshotSummary) => void;
  /** Closes the panel when it slides over the page (narrow windows); the button is hidden otherwise. */
  onClose: () => void;
}

/** The selected snapshot: preview, what it is, where it is stored, and what you can do with it. */
export function SnapshotDetailPanel({
  snapshot,
  game,
  onRestore,
  onDelete,
  onReveal,
  onClose,
}: SnapshotDetailPanelProps) {
  const { t, i18n } = useLingui();
  const labels = useSnapshotLabels();
  const close = (
    <IconButton icon="x" label={t`Close`} size="sm" className="sm-detail-close" onClick={onClose} />
  );
  if (!snapshot) {
    return (
      <aside className="sm-detail" aria-label={t`Snapshot details`}>
        {close}
        <p className="sm-empty-detail">{t`Select a snapshot to see where it is stored and what it contains.`}</p>
      </aside>
    );
  }

  const onPc = snapshot.archivePath !== null && snapshot.status !== "missing_local";
  const verified = snapshot.verifiedAt !== null;
  const rows: [string, string, boolean?][] = [[t`Created`, formatDateTime(snapshot.createdAt, i18n.locale)]];
  if (snapshot.playtimeSeconds !== null) rows.push([t`Playtime`, formatPlaytime(snapshot.playtimeSeconds)]);
  rows.push([t`Size`, labels.contents(snapshot.totalFiles, formatBytes(snapshot.totalBytes, i18n.locale))]);
  rows.push([t`Device`, snapshot.deviceName]);

  const cloudRow = {
    synced: { text: "L8Play", note: snapshot.gameTitle, icon: "checkCircle" as const, label: t`Uploaded`, ok: true },
    uploading: { text: t`Uploading…`, note: null, icon: "cloudUp" as const, label: t`Uploading`, ok: false },
    queued: { text: t`Waiting to upload`, note: null, icon: "clock" as const, label: t`Queued`, ok: false },
    failed: { text: t`Upload failed`, note: null, icon: "alertCircle" as const, label: t`Failed`, ok: false },
    none: null,
  }[snapshot.cloud];

  return (
    <aside className="sm-detail" aria-label={t`Snapshot details`}>
      {close}
      {game && (
        <div className="sm-shot">
          <GameArt game={game} kind="hero" showTitle={false} />
        </div>
      )}
      <div>
        <h2 className="sm-dname">{labels.name(snapshot)}</h2>
        <div className="sm-dsub">
          <StatusBadge status={KIND_BADGE[snapshot.kind]} size="sm">
            {labels.kind(snapshot.kind)}
          </StatusBadge>
          {snapshot.gameTitle}
        </div>
      </div>
      {snapshot.description ? <div className="sm-note">{snapshot.description}</div> : null}
      <dl className="sm-kv">
        {rows.map(([label, value]) => (
          <div key={label} className="contents">
            <dt>{label}</dt>
            <dd>{value}</dd>
          </div>
        ))}
        <dt>SHA-256</dt>
        <dd className="m">{`${shortHash(snapshot.archiveSha256)}${verified ? " ✓" : ""}`}</dd>
      </dl>
      <div className="sm-avail">
        <div className="l8-eyebrow">{t`Stored in`}</div>
        <div className="sm-av">
          <DsIcon name="drive" size={18} className="text-local" />
          <span className="t">
            {t`This PC`}
            <span className="p" title={snapshot.archivePath ?? undefined}>
              {onPc ? snapshot.archivePath : t`Not on this PC: download it to restore`}
            </span>
          </span>
          {onPc ? (
            <DsIcon name="checkCircle" size={18} className="ok" label={verified ? t`Verified` : t`Stored`} />
          ) : (
            <DsIcon name="alert" size={18} className="warn" label={t`Missing`} />
          )}
        </div>
        {cloudRow && (
          <div className="sm-av">
            <DsIcon name="cloud" size={18} className="text-cloud" />
            <span className="t">
              {t`Google Drive`}
              <span className="p">{cloudRow.note ? `${cloudRow.text} / ${cloudRow.note}` : cloudRow.text}</span>
            </span>
            <DsIcon name={cloudRow.icon} size={18} className={cloudRow.ok ? "ok" : "warn"} label={cloudRow.label} />
          </div>
        )}
      </div>
      <div className="sm-dacts">
        <Button variant="primary" icon="restore" block onClick={() => onRestore(snapshot)}>
          {onPc ? t`Restore…` : t`Download and restore…`}
        </Button>
        {onPc && (
          <IconButton
            icon="folderOpen"
            label={t`Show in folder`}
            variant="secondary"
            onClick={() => onReveal(snapshot)}
          />
        )}
        <IconButton icon="trash" label={t`Delete…`} variant="secondary" onClick={() => onDelete(snapshot)} />
      </div>
    </aside>
  );
}
