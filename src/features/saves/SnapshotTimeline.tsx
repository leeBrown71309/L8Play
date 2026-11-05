import { useLingui } from "@lingui/react/macro";

import { DsIcon } from "@/components/ds/ds-icon";
import { StatusBadge, type StatusKind } from "@/components/ds/status-badge";
import { StorageTags, type CloudCopyState } from "@/components/ds/storage-tags";
import { notify } from "@/components/feedback/toast-store";
import { GameArt } from "@/components/game/GameArt";
import { Button } from "@/components/ui/button";
import { useDownloadSnapshot } from "@/features/cloud/use-cloud";
import { useGames } from "@/features/library/use-library";
import { SnapshotMenu, type SnapshotMenuItem } from "@/features/saves/snapshot-menu";
import { useSnapshotLabels } from "@/features/saves/snapshot-labels";
import { useRevealSnapshot, useSetSnapshotPinned, useVerifySnapshot } from "@/features/saves/use-saves";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import type { SnapshotKind } from "@/lib/bindings/SnapshotKind";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";
import { cn } from "@/lib/cn";
import { formatBytes, formatPlaytime } from "@/lib/format";

interface SnapshotTimelineProps {
  snapshots: readonly SnapshotSummary[];
  /** Show the game title on each row (Save Manager). */
  showGame?: boolean;
  /** Snapshot identical to the save on disk. */
  currentId?: string | null;
  selectedId?: string | null;
  /** Smaller thumbnails, actions only on the selected row (Save Manager). */
  compact?: boolean;
  /** Show each snapshot's note under its name. */
  showNote?: boolean;
  onSelect?: (snapshot: SnapshotSummary) => void;
  onRestore: (snapshot: SnapshotSummary) => void;
  onDelete: (snapshot: SnapshotSummary) => void;
}

/** Calendar day key in local time. */
function dayKey(iso: string): string {
  const date = new Date(iso);
  return `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;
}

function dayLabel(iso: string, locale: string, today: string, yesterday: string): string {
  const key = dayKey(iso);
  const now = new Date();
  if (key === dayKey(now.toISOString())) return today;
  if (key === dayKey(new Date(now.getTime() - 86_400_000).toISOString())) return yesterday;
  return new Intl.DateTimeFormat(locale, { weekday: "short", day: "numeric", month: "short" }).format(new Date(iso));
}

const KIND_STATUS: Record<SnapshotKind, StatusKind> = {
  manual: "manual",
  auto_exit: "auto",
  auto_launch: "auto",
  scheduled: "auto",
  safety: "safety",
  conflict_copy: "conflict",
};

function kindClass(kind: SnapshotKind): string {
  if (kind === "manual") return "kind-manual";
  if (kind === "safety") return "kind-safety";
  return "kind-auto";
}

function cloudCopy(cloud: SnapshotSummary["cloud"]): CloudCopyState {
  if (cloud === "synced") return true;
  if (cloud === "uploading") return "uploading";
  if (cloud === "queued") return "pending";
  return false;
}

interface SnapshotRowProps extends Pick<SnapshotTimelineProps, "showGame" | "compact" | "showNote"> {
  onSelect: ((snapshot: SnapshotSummary) => void) | undefined;
  snapshot: SnapshotSummary;
  game: GameSummary | undefined;
  selected: boolean;
  current: boolean;
  onRestore: (snapshot: SnapshotSummary) => void;
  onDelete: (snapshot: SnapshotSummary) => void;
}

/** One snapshot: what (name and kind), when, how big, where (PC / Drive) and its actions. */
function SnapshotRow({
  snapshot,
  game,
  selected,
  current,
  showGame,
  compact,
  showNote,
  onSelect,
  onRestore,
  onDelete,
}: SnapshotRowProps) {
  const { t, i18n } = useLingui();
  const labels = useSnapshotLabels();
  const pin = useSetSnapshotPinned();
  const verify = useVerifySnapshot();
  const reveal = useRevealSnapshot();
  const download = useDownloadSnapshot();

  // A copy that only lives in Drive is downloaded (and checksum-verified) when restored.
  const cloudOnly = snapshot.status === "missing_local" && snapshot.cloud === "synced";
  const usable = snapshot.status === "ready" || cloudOnly;
  const local = snapshot.status !== "missing_local";

  const onVerify = () =>
    verify.mutate(snapshot.id, {
      onSuccess: () =>
        notify({ tone: "success", title: t`Snapshot verified`, body: t`Every file matches its checksum.` }),
      onError: (error) => notify({ tone: "danger", title: t`Verification failed`, body: labels.error(error) }),
    });

  const onDownload = () =>
    download.mutate(snapshot.id, {
      onSuccess: () => notify({ tone: "success", title: t`Downloaded to this PC`, body: labels.name(snapshot) }),
      onError: (error) => notify({ tone: "danger", title: t`The download failed`, body: labels.error(error) }),
    });

  const menu: SnapshotMenuItem[] = [
    ...(cloudOnly
      ? [{ id: "restore", label: t`Restore…`, icon: "restore" as const, onSelect: () => onRestore(snapshot) }]
      : []),
    {
      id: "pin",
      label: snapshot.pinned ? t`Allow automatic deletion` : t`Never delete automatically`,
      icon: "lock",
      onSelect: () => pin.mutate({ snapshotId: snapshot.id, pinned: !snapshot.pinned }),
    },
    ...(local ? [{ id: "verify", label: t`Check integrity`, icon: "shieldCheck" as const, onSelect: onVerify }] : []),
    ...(local
      ? [
          {
            id: "reveal",
            label: t`Show in Explorer`,
            icon: "folderOpen" as const,
            onSelect: () => reveal.mutate(snapshot.id),
          },
        ]
      : []),
    { id: "delete", label: t`Delete…`, icon: "trash", danger: true, divider: true, onSelect: () => onDelete(snapshot) },
  ];

  const time = new Intl.DateTimeFormat(i18n.locale, { hour: "2-digit", minute: "2-digit" }).format(
    new Date(snapshot.createdAt),
  );
  const name = labels.name(snapshot);

  return (
    <article
      tabIndex={0}
      aria-label={name}
      onClick={() => onSelect?.(snapshot)}
      onKeyDown={(event) => event.key === "Enter" && onSelect?.(snapshot)}
      className={cn(
        "l8-snap",
        kindClass(snapshot.kind),
        selected && "is-selected",
        compact && "is-compact",
        onSelect && "cursor-pointer",
      )}
    >
      <div className="l8-snap-node">
        <i />
      </div>
      <div className="l8-snap-shot">{game ? <GameArt game={game} kind="hero" showTitle={false} /> : null}</div>
      <div className="l8-snap-main">
        <div className="l8-snap-name">
          <span className="l8-snap-title">{name}</span>
          <StatusBadge status={KIND_STATUS[snapshot.kind]} size="sm">
            {labels.kind(snapshot.kind)}
          </StatusBadge>
          {snapshot.pinned ? (
            <DsIcon name="lock" size={14} className="shrink-0 text-cream" label={t`Never deleted automatically`} />
          ) : null}
          {current ? (
            <StatusBadge status="verified" size="sm">
              {t`Current`}
            </StatusBadge>
          ) : null}
          {snapshot.status === "corrupt" ? (
            <StatusBadge status="error" size="sm">
              {t`Damaged`}
            </StatusBadge>
          ) : null}
          {snapshot.status === "missing_local" && !cloudOnly ? (
            <StatusBadge status="unprotected" size="sm">
              {t`File missing`}
            </StatusBadge>
          ) : null}
        </div>
        <div className="l8-snap-meta">
          {showGame ? <span className="l8-snap-game">{snapshot.gameTitle}</span> : null}
          <span>{time}</span>
          <span>{formatBytes(snapshot.totalBytes, i18n.locale)}</span>
          {snapshot.playtimeSeconds !== null ? (
            <span>{t`${formatPlaytime(snapshot.playtimeSeconds)} played`}</span>
          ) : null}
        </div>
        {snapshot.description && showNote !== false ? <div className="l8-snap-note">{snapshot.description}</div> : null}
      </div>
      <div className="l8-snap-where">
        <StorageTags local={local} cloud={cloudCopy(snapshot.cloud)} />
        {snapshot.cloud === "failed" ? (
          <StatusBadge status="error" size="sm" className="ml-1">
            {t`Upload failed`}
          </StatusBadge>
        ) : null}
      </div>
      <div className="l8-snap-actions">
        {cloudOnly ? (
          <Button
            variant={selected ? "primary" : "secondary"}
            size="sm"
            icon="cloudDown"
            loading={download.isPending}
            onClick={(event) => {
              event.stopPropagation();
              onDownload();
            }}
          >
            {t`Download`}
          </Button>
        ) : (
          <Button
            variant={selected ? "primary" : "secondary"}
            size="sm"
            icon="restore"
            disabled={!usable}
            onClick={(event) => {
              event.stopPropagation();
              onRestore(snapshot);
            }}
          >
            {t`Restore`}
          </Button>
        )}
        <SnapshotMenu items={menu} />
      </div>
    </article>
  );
}

/** Snapshots grouped by day along a rail, newest first, with restore, download and a more menu. */
export function SnapshotTimeline({
  snapshots,
  showGame = false,
  currentId,
  selectedId,
  compact = false,
  showNote = true,
  onSelect,
  onRestore,
  onDelete,
}: SnapshotTimelineProps) {
  const { t, i18n } = useLingui();
  const games = useGames().data;

  const groups: { key: string; label: string; items: SnapshotSummary[] }[] = [];
  for (const snapshot of snapshots) {
    const key = dayKey(snapshot.createdAt);
    const last = groups.at(-1);
    if (last?.key === key) last.items.push(snapshot);
    else
      groups.push({ key, label: dayLabel(snapshot.createdAt, i18n.locale, t`Today`, t`Yesterday`), items: [snapshot] });
  }

  return (
    <div className="l8-timeline">
      {groups.map((group) => (
        <section key={group.key} className="l8-timeline-day">
          <h3 className="l8-timeline-label">
            {group.label}
            <span>{group.items.length === 1 ? t`1 snapshot` : t`${group.items.length} snapshots`}</span>
          </h3>
          {group.items.map((snapshot) => (
            <SnapshotRow
              key={snapshot.id}
              snapshot={snapshot}
              game={games?.find((entry) => entry.id === snapshot.gameId)}
              selected={snapshot.id === selectedId}
              current={snapshot.id === currentId}
              showGame={showGame}
              compact={compact}
              showNote={showNote}
              onSelect={onSelect}
              onRestore={onRestore}
              onDelete={onDelete}
            />
          ))}
        </section>
      ))}
    </div>
  );
}
