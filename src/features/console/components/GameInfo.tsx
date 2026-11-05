import { useLingui } from "@lingui/react/macro";

import { usePlatformNames, PlatformIcon } from "@/components/game/PlatformTag";
import { Button } from "@/components/ui/button";
import { useGameProtection } from "@/features/library/use-game-protection";
import { useGame, useRunningGames } from "@/features/library/use-library";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import { formatPlaytime, formatRelative } from "@/lib/format";

interface GameInfoProps {
  game: GameSummary;
  /** Starts the game (or opens its install page). */
  onPlay: () => void;
  /** Hides the call to action while a panel covers it. */
  showAction?: boolean;
}

/** Title, one line of facts, how well the saves are protected, and the call to action. */
export function GameInfo({ game, onPlay, showAction = true }: GameInfoProps) {
  const { t, i18n } = useLingui();
  const names = usePlatformNames();
  const detail = useGame(game.id).data;
  const running = useRunningGames().has(game.id);
  const protection = useGameProtection();
  const status = protection.of(game.id);
  const newest = protection.snapshots
    .filter((snapshot) => snapshot.gameId === game.id)
    .map((snapshot) => snapshot.createdAt)
    .sort()
    .at(-1);

  const played = game.playtimeSeconds >= 60 ? t`Played for ${formatPlaytime(game.playtimeSeconds)}` : t`Never played`;
  const protectionText = !game.installed
    ? t`Not installed on this PC`
    : status.snapshotCount === 0
      ? t`No snapshot yet`
      : status.local && status.cloud === true
        ? t`Backed up on this PC and in Google Drive`
        : status.local
          ? t`Backed up on this PC`
          : t`Snapshots are waiting to be restored from Drive`;

  return (
    <div className="cs-info">
      <h1 className="cs-title">{game.title}</h1>
      <div className="cs-meta">
        <span className="cs-meta-platform">
          <PlatformIcon platform={game.platform} size={22} />
          {names[game.platform] ?? game.platform}
        </span>
        {detail?.developer ? <span>{t`By ${detail.developer}`}</span> : null}
        <span>{played}</span>
        {game.lastPlayedAt ? <span>{t`Last played ${formatRelative(game.lastPlayedAt, i18n.locale)}`}</span> : null}
      </div>
      <p className="cs-protection">
        {protectionText}
        {newest && game.installed ? ` · ${t`last snapshot ${formatRelative(newest, i18n.locale)}`}` : ""}
      </p>
      {showAction ? (
        <Button
          variant="play"
          size="xl"
          icon={game.installed ? "play" : "download"}
          className="cs-cta"
          disabled={running}
          onClick={onPlay}
        >
          {running ? t`Running` : game.installed ? t`Play` : t`Install`}
        </Button>
      ) : null}
    </div>
  );
}
