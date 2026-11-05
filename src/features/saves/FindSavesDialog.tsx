import { useLingui } from "@lingui/react/macro";
import { useQuery } from "@tanstack/react-query";

import { Skeleton } from "@/components/feedback/Skeleton";
import { notify } from "@/components/feedback/toast-store";
import { FolderPlusIcon, ScanIcon, WarningIcon } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Dialog, Kbd } from "@/components/ui/dialog";
import { useSnapshotLabels } from "@/features/saves/snapshot-labels";
import { useAddSaveFolder } from "@/features/saves/use-saves";
import type { CandidateReason } from "@/lib/bindings/CandidateReason";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import type { SaveCandidate } from "@/lib/bindings/SaveCandidate";
import { formatBytes, formatRelative } from "@/lib/format";
import { findSaves, openWikiPage } from "@/lib/tauri/commands";

interface FindSavesDialogProps {
  game: GameSummary;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/** Folders proposed by PCGamingWiki and a local search. Nothing is backed up until one is chosen. */
export function FindSavesDialog({ game, open, onOpenChange }: FindSavesDialogProps) {
  const { t, i18n } = useLingui();
  const labels = useSnapshotLabels();
  const add = useAddSaveFolder();
  const search = useQuery({
    queryKey: ["saves", "find", game.id],
    queryFn: () => findSaves(game.id),
    enabled: open,
    staleTime: 0,
    gcTime: 0,
    retry: false,
  });

  const reasonText = (reason: CandidateReason): string =>
    ({
      wiki_template: t`Listed on PCGamingWiki`,
      folder_name_matches_title: t`Folder named like the game`,
      folder_name_matches_install: t`Folder named like the install folder`,
      install_save_folder: t`Save folder inside the install folder`,
    })[reason];

  const use = (candidate: SaveCandidate) =>
    add.mutate(
      { gameId: game.id, folder: candidate.path },
      {
        onSuccess: () => {
          notify({ tone: "success", title: t`Save folder added`, body: candidate.path });
          void search.refetch();
        },
        onError: (error) => notify({ tone: "danger", title: t`This folder was not added`, body: labels.error(error) }),
      },
    );

  const outcome = search.data;
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      width={680}
      icon={<ScanIcon size={22} />}
      title={t`Find saves`}
      description={game.title}
      hint={
        <>
          <Kbd>Esc</Kbd> {t`close`}
        </>
      }
      actions={
        <Button variant="primary" onClick={() => onOpenChange(false)}>
          {t`Done`}
        </Button>
      }
    >
      <p className="m-0 text-[13px] leading-[19px] text-ink-muted">
        {t`These folders exist on this PC and look like this game’s saves. Check the evidence, then add the ones that are right. Nothing is backed up until you do.`}
      </p>

      {search.isFetching && <Skeleton className="h-28 rounded-md" />}
      {search.isError && (
        <p className="m-0 flex items-start gap-2 rounded-md bg-warning-soft p-3 text-[13px] text-ink">
          <WarningIcon size={16} className="mt-0.5 shrink-0 text-warning" />
          {t`The search could not run. Check the log file for details.`}
        </p>
      )}
      {outcome?.wiki === "offline" && !search.isFetching && (
        <p className="m-0 flex items-start gap-2 rounded-md bg-warning-soft p-3 text-[13px] text-ink">
          <WarningIcon size={16} className="mt-0.5 shrink-0 text-warning" />
          {t`PCGamingWiki could not be reached, so only this PC was searched.`}
        </p>
      )}

      {outcome && !search.isFetching && outcome.candidates.length === 0 && outcome.wiki !== "offline" && (
        <p className="m-0 rounded-md bg-surface-100 p-4 text-[13.5px] leading-5 text-ink-muted">
          {outcome.wiki === "found"
            ? t`PCGamingWiki lists save folders for this game, but none exists on this PC yet. Play the game once, then search again.`
            : t`Nothing found. Add the save folder yourself if you know where the game keeps it.`}
        </p>
      )}

      {!search.isFetching && outcome && outcome.candidates.length > 0 && (
        <ul className="m-0 flex max-h-[320px] list-none flex-col gap-2 overflow-y-auto p-0">
          {outcome.candidates.map((candidate) => (
            <li key={candidate.path} className="flex items-start gap-3 rounded-md bg-surface-100 p-3">
              <div className="min-w-0 flex-1">
                <div className="break-all font-mono text-xs text-ink select-text">{candidate.path}</div>
                <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-0.5 text-[12px] text-ink-faint">
                  <span className="font-semibold text-flare-text">{reasonText(candidate.reason)}</span>
                  <span>{labels.contents(candidate.fileCount, formatBytes(candidate.totalBytes, i18n.locale))}</span>
                  {candidate.newestModifiedAt && (
                    <span>{t`changed ${formatRelative(candidate.newestModifiedAt, i18n.locale)}`}</span>
                  )}
                </div>
                <div className="mt-1 text-[11.5px] break-all text-ink-faint">
                  {t`Evidence:`} <span className="font-mono">{candidate.evidence}</span>
                  {candidate.evidenceUrl && (
                    <>
                      {" · "}
                      <button
                        type="button"
                        className="font-semibold text-flare-text underline-offset-2 hover:underline"
                        onClick={() => void openWikiPage(candidate.evidenceUrl ?? "")}
                      >
                        {t`Read the wiki page`}
                      </button>
                    </>
                  )}
                </div>
              </div>
              <Button
                size="sm"
                variant="primary"
                icon={<FolderPlusIcon size={15} />}
                disabled={add.isPending}
                onClick={() => use(candidate)}
              >
                {t`Use this folder`}
              </Button>
            </li>
          ))}
        </ul>
      )}
    </Dialog>
  );
}
