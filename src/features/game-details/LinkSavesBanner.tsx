import { useLingui } from "@lingui/react/macro";
import { useNavigate } from "react-router";

import { Banner } from "@/components/ds/page-blocks";
import { notify } from "@/components/feedback/toast-store";
import { Button } from "@/components/ui/button";
import { useLinkArchivedGame, useLinkCandidates } from "@/features/library/use-library";
import type { GameSummary } from "@/lib/bindings/GameSummary";

/**
 * Shown on a saves-only game once a game with the same title has been installed: joining them moves the
 * imported snapshots to the real game, so they restore from its own save folder.
 */
export function LinkSavesBanner({ game }: { game: GameSummary }) {
  const { t } = useLingui();
  const navigate = useNavigate();
  const candidates = useLinkCandidates(game.id, game.archived);
  const link = useLinkArchivedGame();
  const target = candidates.data?.[0];
  if (!game.archived || !target) return null;

  const join = () =>
    link.mutate(
      { archivedId: game.id, targetId: target.id },
      {
        onSuccess: (joined) => {
          notify({ tone: "success", title: t`Saves joined to ${joined.title}` });
          void navigate(`/games/${joined.id}`, { replace: true });
        },
        onError: () => notify({ tone: "danger", title: t`The saves could not be joined` }),
      },
    );

  return (
    <Banner
      tone="success"
      icon="merge"
      title={t`${target.title} is installed now`}
      action={
        <Button variant="primary" icon="merge" loading={link.isPending} onClick={join}>
          {t`Join the saves`}
        </Button>
      }
    >
      {t`Move these imported snapshots to the installed game, so they restore into its own save folder.`}
    </Banner>
  );
}
