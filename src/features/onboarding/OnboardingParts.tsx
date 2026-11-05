import { useLingui } from "@lingui/react/macro";

import { DsIcon } from "@/components/ds/ds-icon";
import { ProgressBar } from "@/components/ds/progress-bar";
import { GameArt } from "@/components/game/GameArt";
import { PlatformIcon, usePlatformNames } from "@/components/game/PlatformTag";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import type { Platform } from "@/lib/bindings/Platform";
import type { PlatformScan } from "@/lib/bindings/PlatformScan";

const WALL_SIZE = 15;
const LAUNCHERS: readonly Platform[] = ["steam", "epic", "xbox", "gog"];

/** The rotated cover wall behind the right panel. Missing covers become soft empty tiles. */
export function CoverWall({ games, dimmed }: { games: readonly GameSummary[]; dimmed: boolean }) {
  const tiles = Math.max(WALL_SIZE, Math.min(games.length, 20));
  return (
    <div className="ob-wall" aria-hidden="true">
      {Array.from({ length: tiles }, (_, index) => {
        const game = games[index];
        return game ? (
          <div key={game.id} style={dimmed && index > 12 ? { opacity: 0.35 } : undefined}>
            <GameArt game={game} kind="cover" />
          </div>
        ) : (
          <div key={`empty-${index}`} className="empty" />
        );
      })}
    </div>
  );
}

interface ScanGlassProps {
  games: number;
  saveFolders: number;
  artworkSets: number;
  scanning: boolean;
}

/** The translucent progress card shown over the wall while the library is scanned. */
export function ScanGlass({ games, saveFolders, artworkSets, scanning }: ScanGlassProps) {
  const { t } = useLingui();
  return (
    <div className="ob-glass">
      <b>{scanning ? t`Looking for games…` : t`Found ${games} games`}</b>
      <span>{t`Matching artwork from SteamGridDB…`}</span>
      <div className="ob-stats">
        <div>
          <b>{games}</b>
          <br />
          {t`games`}
        </div>
        <div>
          <b>{saveFolders}</b>
          <br />
          {t`save folders`}
        </div>
        <div>
          <b>{artworkSets}</b>
          <br />
          {t`artwork sets`}
        </div>
      </div>
      <ProgressBar tone="cream" size="sm" {...(scanning ? { indeterminate: true } : { value: 100 })} />
    </div>
  );
}

interface PlatformListProps {
  games: readonly GameSummary[];
  scan: readonly PlatformScan[] | null;
}

/** One row per game launcher: found (with a game count) or not installed on this PC. */
export function PlatformList({ games, scan }: PlatformListProps) {
  const { t } = useLingui();
  const names = usePlatformNames();
  const platforms: Platform[] = [...LAUNCHERS];
  // Anything else the backend reports (or the library already holds) is listed too.
  for (const entry of scan ?? []) if (!platforms.includes(entry.platform)) platforms.push(entry.platform);
  for (const game of games)
    if (game.platform !== "standalone" && !platforms.includes(game.platform)) platforms.push(game.platform);
  const standalone = games.filter((game) => game.platform === "standalone").length;

  return (
    <div className="ob-list" role="list">
      {platforms.map((platform) => {
        const inLibrary = games.filter((game) => game.platform === platform).length;
        const reported = scan?.find((entry) => entry.platform === platform);
        // `found: null` means the launcher is not installed; before any scan the library count speaks.
        const present = reported ? reported.found !== null : inLibrary > 0;
        const count = reported?.found ?? inLibrary;
        return (
          <div key={platform} role="listitem" className={present ? undefined : "off"}>
            <PlatformIcon platform={platform} size={20} />
            <span className="grow">{names[platform] ?? platform}</span>
            <span className="count">
              {present ? (count === 1 ? t`1 game` : t`${count} games`) : scan ? t`Not found` : t`Not scanned yet`}
            </span>
          </div>
        );
      })}
      <div role="listitem">
        <PlatformIcon platform="standalone" size={20} />
        <span className="grow">{t`Added by hand`}</span>
        <span className="count">{standalone === 1 ? t`1 game` : t`${standalone} games`}</span>
      </div>
    </div>
  );
}

/** What is protected, in the design system's "done" list. */
export function DoneList({ items }: { items: readonly string[] }) {
  return (
    <div className="ob-list">
      {items.map((item) => (
        <div key={item}>
          <DsIcon name="checkCircle" size={20} className="ok" />
          {item}
        </div>
      ))}
    </div>
  );
}
