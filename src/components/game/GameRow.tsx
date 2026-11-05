import { useLingui } from "@lingui/react/macro";
import { useNavigate } from "react-router";

import { DsIcon } from "@/components/ds/ds-icon";
import { Protection } from "@/components/ds/storage-tags";
import { GameArt } from "@/components/game/GameArt";
import { PlatformIcon, usePlatformNames } from "@/components/game/PlatformTag";
import { Button } from "@/components/ui/button";
import type { GameProtection } from "@/features/library/game-protection";
import type { LibrarySort } from "@/features/library/library-view-store";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import { cn } from "@/lib/cn";
import { formatBytes, formatPlaytime, formatRelative } from "@/lib/format";

interface GameRowProps {
  game: GameSummary;
  running: boolean;
  loading?: boolean;
  protection: GameProtection;
  onPlay: (game: GameSummary) => void;
}

/** List-view row (`l8-grow`): icon, title, platform, last played, playtime, size, saves, action. */
export function GameRow({ game, running, loading = false, protection, onPlay }: GameRowProps) {
  const { t, i18n } = useLingui();
  const navigate = useNavigate();
  const platformNames = usePlatformNames();
  const open = () => void navigate(`/games/${game.id}`);
  return (
    <div
      role="row"
      tabIndex={0}
      className={cn("l8-grow cursor-pointer", !game.installed && "is-uninstalled")}
      aria-busy={loading}
      onClick={open}
      onKeyDown={(event) => {
        if (event.target === event.currentTarget && event.key === "Enter") open();
      }}
    >
      <div role="cell" className="l8-grow-icon">
        <GameArt game={game} kind="icon" showTitle={false} loading={loading} />
      </div>
      <div role="cell" className="l8-grow-title">
        <div className="l8-grow-name">
          {game.title}
          {game.favorite ? <DsIcon name="starFill" size={14} className="l8-grow-fav" label={t`Favourite`} /> : null}
        </div>
        <div className="l8-grow-sub truncate font-mono text-[11.5px]">{game.installPath ?? "—"}</div>
      </div>
      <div role="cell" className="l8-grow-cell gap-2 text-[12.5px] text-ink-muted">
        <PlatformIcon platform={game.platform} size={22} className="shrink-0 text-ink" />
        <span className="truncate">{platformNames[game.platform] ?? game.platform}</span>
      </div>
      <div role="cell" className={cn("l8-grow-cell is-muted", running && "font-semibold text-flare-text")}>
        {running ? t`Playing now` : game.lastPlayedAt ? formatRelative(game.lastPlayedAt, i18n.locale) : "—"}
      </div>
      <div role="cell" className="l8-grow-cell is-num">
        {formatPlaytime(game.playtimeSeconds)}
      </div>
      <div role="cell" className="l8-grow-cell is-num">
        {game.installed ? formatBytes(game.sizeBytes, i18n.locale) : <span className="text-ink-faint">—</span>}
      </div>
      <div role="cell" className="l8-grow-cell">
        <Protection local={protection.local} cloud={protection.cloud} />
      </div>
      <div role="cell" className="l8-grow-action">
        <Button
          size="sm"
          variant={game.installed ? "secondary" : "ghost"}
          icon={game.installed ? "play" : "download"}
          onClick={(event) => {
            event.stopPropagation();
            onPlay(game);
          }}
        >
          {game.installed ? t`Play` : t`Install`}
        </Button>
      </div>
    </div>
  );
}

const SORT_COLUMN: Record<LibrarySort, number> = { recent: 3, title: 1, playtime: 4, size: 5, added: -1 };

/** Header row of the list view; the column the list is sorted by is highlighted. */
export function GameRowHeader({ sort }: { sort: LibrarySort }) {
  const { t } = useLingui();
  const columns = ["", t`Title`, t`Platform`, t`Last played`, t`Playtime`, t`Size`, t`Saves`, ""];
  const sorted = SORT_COLUMN[sort];
  return (
    <div role="row" className="l8-grow l8-grow-head">
      {columns.map((label, index) => (
        <div
          key={index}
          role="columnheader"
          className={cn(
            index === 0
              ? "l8-grow-icon"
              : index === 1
                ? "l8-grow-title"
                : index === 7
                  ? "l8-grow-action"
                  : "l8-grow-cell",
            (index === 4 || index === 5) && "is-num",
            index === sorted && "is-sorted",
          )}
        >
          {label}
          {index === sorted ? <DsIcon name="chevronDown" size={14} /> : null}
        </div>
      ))}
    </div>
  );
}
