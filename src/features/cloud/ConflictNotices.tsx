import { useLingui } from "@lingui/react/macro";
import { useState } from "react";

import { Banner } from "@/components/ds/page-blocks";
import { Button } from "@/components/ui/button";
import type { ConflictInfo } from "@/lib/bindings/ConflictInfo";

const HIDDEN_KEY = "l8play.hiddenConflicts";

function readHidden(): number[] {
  try {
    const parsed: unknown = JSON.parse(window.localStorage.getItem(HIDDEN_KEY) ?? "[]");
    return Array.isArray(parsed) ? parsed.filter((id): id is number => typeof id === "number") : [];
  } catch {
    return [];
  }
}

function writeHidden(ids: number[]): void {
  try {
    window.localStorage.setItem(HIDDEN_KEY, JSON.stringify(ids));
  } catch {
    // Storage can be unavailable; the banners then simply come back next time.
  }
}

/**
 * Which conflict banners the user hid. Hiding is only a display choice for this PC: the conflict stays
 * open (both saves are kept) and can be shown again.
 */
function useHiddenConflicts() {
  const [hidden, setHidden] = useState<number[]>(readHidden);
  const update = (next: number[]) => {
    setHidden(next);
    writeHidden(next);
  };
  return {
    hidden,
    hide: (id: number) => update([...new Set([...hidden, id])]),
    showAll: () => update([]),
  };
}

interface ConflictNoticesProps {
  conflicts: readonly ConflictInfo[];
  onResolve: (conflict: ConflictInfo) => void;
}

/** What a conflict is, and one banner per conflict with Resolve and Hide. */
export function ConflictNotices({ conflicts, onResolve }: ConflictNoticesProps) {
  const { t } = useLingui();
  const { hidden, hide, showAll } = useHiddenConflicts();
  if (conflicts.length === 0) return null;

  const visible = conflicts.filter((conflict) => !hidden.includes(conflict.id));
  const hiddenCount = conflicts.length - visible.length;

  return (
    <div className="cloud-banners">
      <Banner tone="neutral" icon="info" title={t`What is a sync conflict?`}>
        {t`It happens when this PC and Google Drive each hold a different latest save of the same game, so L8Play cannot tell which one is right. It usually means the game was played on two PCs, or that L8Play was reinstalled. Nothing is lost: both saves are kept and never cleaned up automatically. Press Resolve to choose one (if you are unsure, choose “Keep both, change nothing yet”), or leave it for later.`}
      </Banner>
      {visible.map((conflict) => (
        <Banner
          key={conflict.id}
          tone="warning"
          icon="merge"
          title={t`${conflict.gameTitle} has two different saves.`}
          action={
            <>
              <Button size="sm" variant="ghost" onClick={() => hide(conflict.id)}>
                {t`Hide`}
              </Button>
              <Button size="sm" variant="primary" onClick={() => onResolve(conflict)}>
                {t`Resolve`}
              </Button>
            </>
          }
        >
          {t`Both saves are kept until you choose. You can hide this notice and decide later.`}
        </Banner>
      ))}
      {hiddenCount > 0 ? (
        <div>
          <Button size="sm" variant="ghost" onClick={showAll}>
            {hiddenCount === 1 ? t`Show 1 hidden conflict` : t`Show ${hiddenCount} hidden conflicts`}
          </Button>
        </div>
      ) : null}
    </div>
  );
}
