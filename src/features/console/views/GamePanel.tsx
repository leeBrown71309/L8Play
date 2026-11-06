import { useLingui } from "@lingui/react/macro";
import { useRef } from "react";

import { DsIcon, type DsIconName } from "@/components/ds/ds-icon";
import { GameArt } from "@/components/game/GameArt";
import { ListRow } from "@/features/console/components/ListRow";
import { useConsoleNav, useZoneFocus } from "@/features/console/core/nav-store";
import { consoleToast } from "@/features/console/core/toast-store";
import { useConsoleGameActions } from "@/features/console/core/use-console-play";
import { useScrollFocused } from "@/features/console/core/use-scroll-focused";
import { useViewInput } from "@/features/console/core/view-input";
import { consoleSound } from "@/features/console/sound";
import { useCloudStatus } from "@/features/cloud/use-cloud";
import { useRunningGames } from "@/features/library/use-library";
import { useSnapshotLabels } from "@/features/saves/snapshot-labels";
import { useCreateSnapshot, useSnapshots } from "@/features/saves/use-saves";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import { cn } from "@/lib/cn";
import { formatPlaytime } from "@/lib/format";

interface Action {
  id: string;
  label: string;
  detail?: string | undefined;
  icon: DsIconName;
  disabled?: boolean | undefined;
  run: () => void;
}

/** The panel that slides in over the library for the selected game: a short list of things to do with it. */
export function GamePanel({ game, closing }: { game: GameSummary; closing: boolean }) {
  const { t } = useLingui();
  const labels = useSnapshotLabels();
  const dispatch = useConsoleNav((state) => state.dispatch);
  const actions = useConsoleGameActions();
  const running = useRunningGames().has(game.id);
  const snapshots = useSnapshots(game.id).data ?? [];
  const connected = useCloudStatus().data?.account?.status === "connected";
  const create = useCreateSnapshot();

  const createSnapshot = () => {
    if (create.isPending) return;
    const name = t`Console snapshot`;
    create.mutate(
      { gameId: game.id, name, description: null, pinned: false, storage: connected ? "both" : "local" },
      {
        onSuccess: (result) => {
          if (result.status === "created") {
            consoleSound.play("launch");
            consoleToast({ tone: "success", title: t`Snapshot created`, body: game.title });
          } else {
            consoleToast({
              tone: "neutral",
              title: result.status === "unchanged" ? t`Already protected` : t`Nothing to back up yet`,
              body: result.status === "unchanged" ? t`Your save has not changed since the last snapshot.` : game.title,
            });
          }
        },
        onError: (error) => {
          consoleSound.play("error");
          consoleToast({ tone: "danger", title: t`The snapshot was not created`, body: labels.error(error) });
        },
      },
    );
  };

  const savesOnly = game.archived;
  const items: Action[] = [
    // A saves-only game has nothing to launch or install: its list starts with the saves.
    ...(savesOnly
      ? []
      : [
          {
            id: "play",
            label: running ? t`Running` : game.installed ? t`Play` : t`Install`,
            detail: game.playtimeSeconds >= 60 ? t`${formatPlaytime(game.playtimeSeconds)} played` : undefined,
            icon: game.installed ? ("play" as const) : ("download" as const),
            disabled: running,
            run: () => actions.play(game),
          },
        ]),
    {
      id: "favourite",
      label: game.favorite ? t`Remove from favourites` : t`Add to favourites`,
      icon: game.favorite ? "starFill" : "star",
      run: () => actions.toggleFavorite(game),
    },
    ...(game.installed || savesOnly
      ? [
          ...(savesOnly
            ? []
            : [
                {
                  id: "create",
                  label: create.isPending ? t`Creating…` : t`Create snapshot`,
                  detail: connected ? t`On this PC and in Google Drive` : t`On this PC`,
                  icon: "snapshot" as const,
                  disabled: create.isPending,
                  run: createSnapshot,
                },
              ]),
          {
            id: "snapshots",
            label: t`Snapshots`,
            detail: snapshots.length === 1 ? t`1 snapshot` : t`${snapshots.length} snapshots`,
            icon: "restore" as const,
            run: () => dispatch({ type: "pushModal", modal: { kind: "game-snapshots" } }),
          },
        ]
      : []),
    {
      id: "artwork",
      label: t`Artwork`,
      detail: t`Change artwork in the desktop interface`,
      icon: "image",
      run: () => consoleToast({ tone: "neutral", title: t`Change artwork in the desktop interface` }),
    },
    { id: "back", label: t`Back`, icon: "chevronLeft", run: () => dispatch({ type: "closeGame" }) },
  ];

  const zone = useZoneFocus("game.actions", items.length);
  const list = useRef<HTMLDivElement>(null);
  useScrollFocused(list, zone.index);
  const confirm = (item: Action | undefined) => {
    if (!item || item.disabled) return;
    consoleSound.play(item.id === "back" ? "back" : "select");
    item.run();
  };

  useViewInput((action) => {
    if (action === "up" || action === "down") {
      const before = zone.index;
      zone.move(action === "down" ? 1 : -1);
      if (before !== Math.min(items.length - 1, Math.max(0, before + (action === "down" ? 1 : -1)))) {
        consoleSound.play("move");
      }
    } else if (action === "confirm") confirm(items[zone.index]);
  }, !closing);

  return (
    <aside className={cn("cs-panel", closing && "is-closing")} aria-label={game.title}>
      <div className="cs-panel-brand">
        <GameArt game={game} kind="logo" className="cs-panel-logo" />
        {game.artwork.logo ? <h2 className="l8-sr">{game.title}</h2> : <h2 className="cs-panel-title">{game.title}</h2>}
      </div>
      <div ref={list} className="cs-panel-list" role="menu">
        {items.map((item, index) => (
          <ListRow
            key={item.id}
            label={item.label}
            detail={item.detail}
            icon={item.icon}
            disabled={item.disabled}
            focused={zone.index === index}
            onFocus={() => zone.set(index)}
            onActivate={() => confirm(item)}
            trailing={item.id === "play" && game.installed ? <DsIcon name="chevronRight" size={22} /> : undefined}
          />
        ))}
      </div>
    </aside>
  );
}
