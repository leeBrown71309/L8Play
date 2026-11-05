import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { Link } from "react-router";

import { ROUTES } from "@/app/routes";
import { DsIcon } from "@/components/ds/ds-icon";
import { SearchField } from "@/components/ds/form-fields";
import { StatusBadge } from "@/components/ds/status-badge";
import { EmptyState } from "@/components/feedback/EmptyState";
import { Skeleton } from "@/components/feedback/Skeleton";
import { SnapshotIcon } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Chip } from "@/components/ui/chip";
import { useCloudStatus } from "@/features/cloud/use-cloud";
import { SaveLocationsCard } from "@/features/saves/SaveLocationsCard";
import { SnapshotTimeline } from "@/features/saves/SnapshotTimeline";
import { useSnapshotLabels } from "@/features/saves/snapshot-labels";
import {
  countByFilter,
  filterSnapshots,
  summarizeCopies,
  type SnapshotFilter,
} from "@/features/saves/snapshot-summary";
import { useCurrentSave, useRevealSnapshot, useSaveStatuses } from "@/features/saves/use-saves";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";
import { formatBytes, formatRelative } from "@/lib/format";

interface GameSavesPanelProps {
  game: GameSummary;
  running: boolean;
  snapshots: readonly SnapshotSummary[] | undefined;
  onCreate: () => void;
  onRestore: (snapshot: SnapshotSummary) => void;
  onDelete: (snapshot: SnapshotSummary) => void;
}

/** Saves tab (design system "Game saves"): filters, the snapshot timeline, and the save-state cards. */
export function GameSavesPanel({ game, running, snapshots, onCreate, onRestore, onDelete }: GameSavesPanelProps) {
  const { t, i18n } = useLingui();
  const labels = useSnapshotLabels();
  const current = useCurrentSave(game.id);
  const status = useSaveStatuses().data?.find((entry) => entry.gameId === game.id);
  const cloud = useCloudStatus().data;
  const reveal = useRevealSnapshot();
  const [filter, setFilter] = useState<SnapshotFilter>("all");
  const [query, setQuery] = useState("");

  const hasLocations = (status?.locationCount ?? 0) > 0;
  const state = current.data;
  const list = snapshots ?? [];
  const counts = countByFilter(list);
  const shown = filterSnapshots(list, filter, query, labels.name);
  const copies = summarizeCopies(list);
  const latest = state?.latestSnapshotId ? list.find((snapshot) => snapshot.id === state.latestSnapshotId) : undefined;
  const connected = cloud?.account?.status === "connected";

  const saveLine = (() => {
    if (!hasLocations) return { warn: true, text: t`Not protected yet` };
    if (!state) return null;
    if (state.latestSnapshotId === null) return { warn: true, text: t`No snapshot yet` };
    if (state.matchesLatest) {
      const when = latest
        ? new Intl.DateTimeFormat(i18n.locale, { hour: "2-digit", minute: "2-digit" }).format(
            new Date(latest.createdAt),
          )
        : "";
      return {
        warn: false,
        text: latest ? t`Identical to “${labels.name(latest)}”, ${when}` : t`Identical to the latest snapshot`,
      };
    }
    return { warn: true, text: t`Changed since the latest snapshot` };
  })();

  return (
    <div className="gd-sv">
      <div>
        <div className="gd-tools">
          <Chip selected={filter === "all"} count={counts.all} onClick={() => setFilter("all")}>
            {t`All`}
          </Chip>
          <Chip selected={filter === "manual"} count={counts.manual} onClick={() => setFilter("manual")}>
            {t`Manual`}
          </Chip>
          <Chip selected={filter === "auto"} count={counts.auto} onClick={() => setFilter("auto")}>
            {t`Auto`}
          </Chip>
          <Chip selected={filter === "safety"} count={counts.safety} onClick={() => setFilter("safety")}>
            {t`Safety copies`}
          </Chip>
          <div className="push">
            <SearchField
              size="sm"
              width={210}
              placeholder={t`Search names and notes`}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
            <Button
              variant="primary"
              icon="plus"
              onClick={onCreate}
              disabled={!hasLocations || running}
              title={running ? t`Close the game first` : undefined}
            >
              {t`Create snapshot`}
            </Button>
          </div>
        </div>

        {snapshots === undefined && <Skeleton className="h-40 rounded-lg" />}
        {snapshots !== undefined && list.length === 0 && (
          <div className="rounded-lg bg-surface-200">
            <EmptyState
              icon={<SnapshotIcon size={30} />}
              title={t`No snapshots yet`}
              body={t`A snapshot is taken automatically each time you stop playing. You can also create one now.`}
            />
          </div>
        )}
        {list.length > 0 && shown.length === 0 && (
          <p className="m-0 rounded-md bg-surface-200 p-4 text-[13.5px] text-ink-muted">
            {t`No snapshot matches this filter.`}
          </p>
        )}
        {shown.length > 0 && (
          <SnapshotTimeline
            snapshots={shown}
            currentId={state?.matchesLatest ? state.latestSnapshotId : null}
            onRestore={onRestore}
            onDelete={onDelete}
          />
        )}
      </div>

      <aside className="gd-sidecol">
        <section className="gd-card">
          <h3>
            {t`Current save`}
            <StatusBadge status="local" size="sm" />
          </h3>
          <div className="gd-cur">
            <span className="ic">
              <DsIcon name="file" size={20} />
            </span>
            <div>
              <b>
                {state && hasLocations
                  ? labels.contents(state.totalFiles, formatBytes(state.totalBytes, i18n.locale))
                  : t`No save found yet`}
              </b>
              {state?.lastModifiedAt ? (
                <span>{t`modified ${formatRelative(state.lastModifiedAt, i18n.locale)}`}</span>
              ) : null}
            </div>
          </div>
          {saveLine ? (
            <div className={saveLine.warn ? "gd-ok is-warn" : "gd-ok"}>
              <DsIcon name={saveLine.warn ? "alert" : "shieldCheck"} size={16} />
              {saveLine.text}
            </div>
          ) : null}
        </section>

        <SaveLocationsCard game={game} unknownToDetector={status?.unknownToLudusavi ?? false} />

        <section className="gd-card">
          <h3>
            {t`Google Drive`}
            {connected ? <StatusBadge status="synced" size="sm" /> : <StatusBadge status="offline" size="sm" />}
          </h3>
          {connected ? (
            <div className="gd-cl">
              <DsIcon name="cloudCheck" size={18} className="text-cloud" />
              <span>
                <Trans>
                  <b>
                    {copies.inDrive} of {copies.total}
                  </b>{" "}
                  snapshots uploaded
                </Trans>
              </span>
            </div>
          ) : (
            <div className="gd-cl">
              <DsIcon name="cloudOff" size={18} />
              <span>
                {t`Not connected.`}{" "}
                <Link to={ROUTES.cloud} className="text-flare-text">
                  {t`Connect Google Drive`}
                </Link>
              </span>
            </div>
          )}
        </section>

        <div className="gd-acts">
          <Button icon="folder" onClick={() => reveal.mutate(null)}>
            {t`Open snapshot folder`}
          </Button>
        </div>
      </aside>
    </div>
  );
}
