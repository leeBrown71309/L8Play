import { useLingui } from "@lingui/react/macro";
import { useMemo, useRef } from "react";

import { ListRow } from "@/features/console/components/ListRow";
import { newestFirst, storedIn } from "@/features/console/components/modals/snapshot-text";
import { dayKey, dayLabel } from "@/features/console/core/day-label";
import { pointerMoved } from "@/features/console/core/pointer-move";
import { useConsoleNav, useZoneFocus } from "@/features/console/core/nav-store";
import { useScrollFocused } from "@/features/console/core/use-scroll-focused";
import { useViewInput } from "@/features/console/core/view-input";
import { consoleSound } from "@/features/console/sound";
import { useSnapshotLabels } from "@/features/saves/snapshot-labels";
import { useSnapshots } from "@/features/saves/use-saves";
import { formatBytes } from "@/lib/format";
import { cn } from "@/lib/cn";

/** Every snapshot grouped by day. The first row filters by game (left and right); confirm opens a snapshot. */
export function SavesView() {
  const { t, i18n } = useLingui();
  const labels = useSnapshotLabels();
  const dispatch = useConsoleNav((state) => state.dispatch);
  const all = useSnapshots(null).data;
  const sorted = useMemo(() => newestFirst(all ?? []), [all]);
  const games = useMemo(() => {
    const seen = new Map<number, string>();
    for (const snapshot of sorted) seen.set(snapshot.gameId, snapshot.gameTitle);
    return [...seen].sort((a, b) => a[1].localeCompare(b[1]));
  }, [sorted]);
  const gameFilter = useZoneFocus("saves.game", games.length + 1);
  const gameId = gameFilter.index === 0 ? null : (games[gameFilter.index - 1]?.[0] ?? null);
  const rows = useMemo(
    () => (gameId === null ? sorted : sorted.filter((snapshot) => snapshot.gameId === gameId)),
    [sorted, gameId],
  );
  const zone = useZoneFocus("saves.list", rows.length + 1);
  const list = useRef<HTMLDivElement>(null);
  useScrollFocused(list, zone.index);
  const where = { pc: t`This PC`, drive: t`Google Drive`, both: t`PC and Drive` };

  const open = (row: number) => {
    const snapshot = rows[row - 1];
    if (!snapshot) return;
    dispatch({ type: "pushModal", modal: { kind: "snapshot-detail", snapshotId: snapshot.id } });
  };

  useViewInput((action) => {
    if (action === "up" || action === "down") {
      const next = Math.min(rows.length, Math.max(0, zone.index + (action === "down" ? 1 : -1)));
      if (next !== zone.index) consoleSound.play("move");
      zone.set(next);
    } else if (zone.index === 0 && (action === "left" || action === "right")) {
      const next = Math.min(games.length, Math.max(0, gameFilter.index + (action === "right" ? 1 : -1)));
      if (next !== gameFilter.index) {
        consoleSound.play("move");
        gameFilter.set(next);
        zone.set(0);
      }
    } else if (action === "confirm") {
      consoleSound.play("select");
      open(zone.index);
    }
  });

  const dated = useMemo(
    () =>
      rows.map((snapshot, index) => {
        const previous = rows[index - 1];
        const newDay = !previous || dayKey(previous.createdAt) !== dayKey(snapshot.createdAt);
        return { snapshot, heading: newDay ? dayLabel(snapshot.createdAt, i18n.locale) : null };
      }),
    [rows, i18n.locale],
  );

  return (
    <section className="cs-section" aria-label={t`Saves`}>
      <h1 className="cs-section-title">{t`Saves`}</h1>
      <div ref={list} className="cs-scroll">
        <div
          className={cn("cs-picker", zone.index === 0 && "is-focused")}
          data-focused={zone.index === 0 || undefined}
          onPointerMove={(event) => pointerMoved(event) && zone.set(0)}
        >
          <span>{t`Game`}</span>
          <b>{gameId === null ? t`All games` : (games.find(([id]) => id === gameId)?.[1] ?? "")}</b>
          <span aria-hidden="true">‹ ›</span>
        </div>
        {rows.length === 0 ? (
          <p className="cs-empty">{t`No snapshot yet. Play a game, or create one from its page.`}</p>
        ) : null}
        {dated.map(({ snapshot, heading }, index) => {
          return (
            <div key={snapshot.id}>
              {heading ? <h2 className="cs-day">{heading}</h2> : null}
              <ListRow
                label={`${snapshot.gameTitle} · ${labels.name(snapshot)}`}
                detail={`${labels.kind(snapshot.kind)} · ${formatBytes(snapshot.archiveSizeBytes, i18n.locale)} · ${storedIn(snapshot, where)}`}
                icon="snapshot"
                focused={zone.index === index + 1}
                onFocus={() => zone.set(index + 1)}
                onActivate={() => open(index + 1)}
              />
            </div>
          );
        })}
      </div>
    </section>
  );
}
