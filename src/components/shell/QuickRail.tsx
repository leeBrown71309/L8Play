import { useLingui } from "@lingui/react/macro";
import { Link } from "react-router";

import { ROUTES } from "@/app/routes";
import { DsIcon } from "@/components/ds/ds-icon";
import { GameArt } from "@/components/game/GameArt";
import { useCloudSummary } from "@/components/shell/use-cloud-summary";
import { useConsoleMode } from "@/features/console/console-mode-store";
import { useArtworkLoading } from "@/features/library/artwork-loading-store";
import { recentlyPlayed } from "@/features/library/filter-games";
import type { GameProtection } from "@/features/library/game-protection";
import { useGameActions } from "@/features/library/use-game-actions";
import { useGameProtection } from "@/features/library/use-game-protection";
import { useGames, useRunningGames } from "@/features/library/use-library";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import { cn } from "@/lib/cn";
import { formatBytes } from "@/lib/format";

const MAX_GAMES = 6;

/** Favourites first, then recently played, installed only. */
export function quickGames(games: readonly GameSummary[]): GameSummary[] {
  const installed = games.filter((game) => game.installed);
  const favorites = recentlyPlayed(
    installed.filter((game) => game.favorite),
    MAX_GAMES,
  ).concat(installed.filter((game) => game.favorite && game.lastPlayedAt === null));
  const rest = recentlyPlayed(
    installed.filter((game) => !game.favorite),
    MAX_GAMES,
  );
  return [...favorites, ...rest].slice(0, MAX_GAMES);
}

/** Dot colour of a rail icon: green once protected, amber while waiting or in conflict, red with no copy. */
export function railDotTone(protection: GameProtection): "success" | "warning" | "danger" {
  if (protection.cloud === "conflict" || protection.cloud === "pending") return "warning";
  return protection.snapshotCount > 0 ? "success" : "danger";
}

const RING_RADIUS = 15;
const RING_LENGTH = 2 * Math.PI * RING_RADIUS;

/** Small donut showing how much of the Google storage is used. */
function StorageRing({ percent, label }: { percent: number; label: string }) {
  const value = Math.max(0, Math.min(100, percent));
  return (
    <svg className="l8-ring" width={44} height={44} viewBox="0 0 40 40" role="img" aria-label={label}>
      <circle cx={20} cy={20} r={RING_RADIUS} className="l8-ring-track" />
      <circle
        cx={20}
        cy={20}
        r={RING_RADIUS}
        className="l8-ring-fill tone-cloud"
        strokeDasharray={`${(RING_LENGTH * value) / 100} ${RING_LENGTH}`}
        transform="rotate(-90 20 20)"
      />
    </svg>
  );
}

/** Quick-launch rail (`l8-qrail`): profile, favourite and recent games with their save state, Drive storage. */
export function QuickRail() {
  const { t, i18n } = useLingui();
  const { data: games } = useGames();
  const running = useRunningGames();
  const actions = useGameActions();
  const protection = useGameProtection();
  const loadingArt = useArtworkLoading((state) => state.pending);
  const cloud = useCloudSummary();
  const quick = games ? quickGames(games) : [];

  const connected = cloud.state !== "none";
  const storageLabel = cloud.usedBytes !== null ? formatBytes(cloud.usedBytes, i18n.locale) : t`Drive`;
  const cloudLabel = !connected ? t`Local` : cloud.state === "attention" ? t`Attention` : storageLabel;
  const cloudTip = !connected ? t`Google Drive is not connected` : cloud.label;

  return (
    <aside aria-label={t`Quick launch`} className="l8-qrail">
      <div className="l8-qrail-panel">
        <button
          type="button"
          className="cs-launch"
          aria-label={t`Console mode`}
          data-tip={t`Console mode (Ctrl+Shift+Enter)`}
          onClick={() => useConsoleMode.getState().enter()}
        >
          <DsIcon name="gamepad" size={22} />
        </button>
        <div className="l8-qrail-sep" />
        {quick.length === 0 ? (
          <span
            tabIndex={0}
            data-tip={t`Your favourite games will appear here`}
            className="grid size-11 place-items-center rounded-full text-ink-faint outline-2 outline-line-strong outline-dashed"
          >
            <DsIcon name="gamepad" size={18} />
          </span>
        ) : null}
        {quick.map((game) => {
          const isRunning = running.has(game.id);
          return (
            <button
              key={game.id}
              type="button"
              className={cn("l8-qgame", isRunning && "is-running")}
              aria-label={t`Play ${game.title}`}
              data-tip={isRunning ? t`${game.title} — running` : game.title}
              onClick={() => actions.play(game)}
            >
              <span className="l8-qgame-art">
                <GameArt game={game} kind="icon" showTitle={false} loading={loadingArt.has(game.id)} />
              </span>
              <i className={cn("l8-qgame-dot", `tone-${railDotTone(protection.of(game.id))}`)} />
              {isRunning ? <span className="l8-qgame-tag">{t`Running`}</span> : null}
            </button>
          );
        })}
      </div>
      <div className="l8-qrail-panel l8-qrail-cloud">
        <Link to={ROUTES.cloud} className="l8-qcloud" aria-label={cloudTip} data-tip={cloudTip}>
          {connected && cloud.usedPercent !== null ? (
            <StorageRing percent={cloud.usedPercent} label={cloudTip} />
          ) : null}
          <DsIcon
            name={
              connected
                ? cloud.state === "syncing"
                  ? "cloudUp"
                  : cloud.state === "attention"
                    ? "alert"
                    : "cloudCheck"
                : "cloud"
            }
            size={18}
            className={cn(cloud.state === "syncing" && "l8-pulse", !connected && "text-ink-faint")}
          />
        </Link>
        <span className="l8-qcloud-label">{cloudLabel}</span>
      </div>
    </aside>
  );
}
