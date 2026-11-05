import { useMemo } from "react";

import { useCloudStatus, useConflicts } from "@/features/cloud/use-cloud";
import { NO_PROTECTION, protectionByGame, type GameProtection } from "@/features/library/game-protection";
import { useSnapshots } from "@/features/saves/use-saves";

/** Protection of every game, and a lookup that defaults to "not protected". */
export function useGameProtection() {
  const snapshots = useSnapshots(null).data;
  const hasAccount = useCloudStatus().data?.account != null;
  const conflicts = useConflicts(hasAccount).data;
  const map = useMemo(
    () => protectionByGame(snapshots ?? [], new Set((conflicts ?? []).map((conflict) => conflict.gameId))),
    [snapshots, conflicts],
  );
  return {
    snapshots: snapshots ?? [],
    of: (gameId: number): GameProtection => map.get(gameId) ?? NO_PROTECTION,
  };
}
